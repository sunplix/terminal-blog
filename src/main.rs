mod auth;
mod captcha;
mod command;
mod db;
mod logger;
mod vfs;

use actix_cors::Cors;
use actix_web::{web, App, HttpServer};
use dotenv::dotenv;
use sqlx::{postgres::PgPoolOptions, PgPool};
use std::fs;
use std::io;

use auth::AuthManager;
use captcha::{get_captcha, CaptchaManager};
use command::{handle_command, CommandRegistry};
use log::info;
use vfs::{PostgresBackend, VfsManager};

// 应用状态
struct AppState {
    db: PgPool,
    auth_manager: AuthManager,
    captcha_manager: CaptchaManager,
    command_registry: CommandRegistry,
    vfs_manager: VfsManager<PostgresBackend>,
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // 加载 .env
    dotenv().ok();

    // 创建日志目录
    let log_dir = std::path::Path::new("logs");
    if !log_dir.exists() {
        fs::create_dir_all(log_dir)?;
    }

    // 初始化日志系统
    let log_path = log_dir.join("app.log");
    if let Err(e) = logger::Logger::init(&log_path) {
        eprintln!("初始化日志系统失败: {}", e);
    }

    info!("应用程序启动");

    // 连接数据库
    let db_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&db_url)
        .await
        .map_err(|e| io::Error::new(io::ErrorKind::Other, format!("数据库连接错误: {}", e)))?;

    // 初始化数据库
    db::initialize_db(pool.clone())
        .await
        .map_err(|e| io::Error::new(io::ErrorKind::Other, format!("数据库初始化错误: {}", e)))?;

    // 创建VFS管理器
    let backend = PostgresBackend::new(pool.clone());
    let vfs_manager = VfsManager::new(backend);

    let app_state = web::Data::new(AppState {
        db: pool,
        auth_manager: AuthManager::new(),
        captcha_manager: CaptchaManager::new(),
        command_registry: CommandRegistry::new(),
        vfs_manager,
    });

    // 启动验证码清理任务
    let cleanup_state = app_state.clone();
    tokio::spawn(async move {
        cleanup_state.captcha_manager.cleanup_expired_captchas();
    });

    // 启动 token 黑名单清理任务
    let token_cleanup_state = app_state.clone();
    tokio::spawn(async move {
        token_cleanup_state.auth_manager.cleanup_expired_tokens();
    });

    println!("服务器启动在 http://0.0.0.0:8080");

    HttpServer::new(move || {
        App::new()
            .app_data(app_state.clone())
            .wrap(
                Cors::default()
                    .allow_any_origin()
                    .allow_any_method()
                    .allow_any_header()
                    .max_age(3600),
            )
            .route("/api/command", web::post().to(handle_command))
            .route("/api/captcha", web::get().to(get_captcha))
    })
    .bind(("0.0.0.0", 8080))?
    .run()
    .await
}
