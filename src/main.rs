use actix_cors::Cors;
use actix_web::{http::header, middleware, web, App, HttpResponse, HttpServer, Responder, Result};
use bcrypt::{hash, verify, DEFAULT_COST};
use chrono::{DateTime, Utc};
use dotenv::dotenv;
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_json;
use serde_json::json;
use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;
use std::collections::HashMap;
use std::env;
use std::fs;
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;

mod auth;
mod captcha;
mod command;
mod logger;
mod vfs;

use auth::{generate_token, AuthManager, Claims, LoginAttempt};
use captcha::{get_captcha, CaptchaManager};
use command::{handle_command, CommandRegistry};
use log::{error, info};
use vfs::{
    model::{Role, User as VfsUser, VfsError},
    storage::backend::PostgresBackend,
    VfsManager,
};

// 命令响应结构体
#[derive(Debug, Serialize)]
struct CommandResponse {
    success: bool,
    message: String,
    data: Option<serde_json::Value>,
}

// 用户结构体
#[derive(Debug, Serialize, Deserialize, Clone)]
struct User {
    id: String,
    username: String,
    password_hash: String,
    email: Option<String>,
    gender: Option<String>,
    birthday: Option<chrono::NaiveDate>,
    role: String,
    created_at: DateTime<Utc>,
}

// 用户注册请求
#[derive(Debug, Deserialize)]
struct RegisterRequest {
    username: String,
    password: String,
}

// 用户登录请求
#[derive(Debug, Deserialize)]
struct LoginRequest {
    username: String,
    password: String,
}

// 用户信息更新请求
#[derive(Debug, Deserialize)]
struct UpdateProfileRequest {
    email: Option<String>,
    gender: Option<String>,
    birthday: Option<String>,
}

// 应用状态
struct AppState {
    db: PgPool,
    auth_manager: AuthManager,
    captcha_manager: CaptchaManager,
    command_registry: CommandRegistry,
    vfs_manager: VfsManager<PostgresBackend>,
}

// 用户名验证
fn validate_username(username: &str) -> Result<(), String> {
    if username.len() < 3 || username.len() > 20 {
        return Err("用户名长度必须在3-20个字符之间".to_string());
    }
    let username_regex = Regex::new(r"^[a-zA-Z0-9_-]+$").unwrap();
    if !username_regex.is_match(username) {
        return Err("用户名只能包含字母、数字、下划线和连字符".to_string());
    }
    Ok(())
}

// 密码强度验证
fn validate_password(password: &str) -> Result<(), String> {
    if password.len() < 8 {
        return Err("密码长度必须至少为8个字符".to_string());
    }
    let has_uppercase = password.chars().any(|c| c.is_uppercase());
    let has_lowercase = password.chars().any(|c| c.is_lowercase());
    let has_digit = password.chars().any(|c| c.is_numeric());
    let has_special = password.chars().any(|c| !c.is_alphanumeric());
    if !has_uppercase {
        return Err("密码必须包含大写字母".to_string());
    }
    if !has_lowercase {
        return Err("密码必须包含小写字母".to_string());
    }
    if !has_digit {
        return Err("密码必须包含数字".to_string());
    }
    if !has_special {
        return Err("密码必须包含特殊字符".to_string());
    }
    Ok(())
}

// 将原来的get_captcha函数重命名为handle_get_captcha
async fn handle_get_captcha(data: web::Data<AppState>) -> impl Responder {
    get_captcha(data).await
}

// 验证邮箱格式
fn validate_email(email: &str) -> Result<(), String> {
    let email_regex = Regex::new(r"^[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}$").unwrap();
    if !email_regex.is_match(email) {
        return Err("邮箱格式不正确".to_string());
    }
    Ok(())
}

// 验证性别
fn validate_gender(gender: &str) -> Result<(), String> {
    match gender {
        "male" | "female" | "other" => Ok(()),
        _ => Err("性别必须是 male、female 或 other".to_string()),
    }
}

// 验证生日
fn validate_birthday(birthday: &str) -> Result<chrono::NaiveDate, String> {
    match chrono::NaiveDate::parse_from_str(birthday, "%Y-%m-%d") {
        Ok(date) => {
            let today = chrono::Utc::now().date_naive();
            if date > today {
                return Err("生日不能是未来的日期".to_string());
            }
            Ok(date)
        }
        Err(_) => Err("生日格式必须是 YYYY-MM-DD".to_string()),
    }
}

// 验证验证码
fn verify_captcha(data: &web::Data<AppState>, session_id: &str, code: &str) -> bool {
    println!("验证验证码 - 会话ID: {}, 验证码: {}", session_id, code);
    data.captcha_manager.verify_captcha(session_id, code)
}

// 定期清理过期验证码
async fn cleanup_expired_captchas(data: web::Data<AppState>) {
    let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(600)); // 每10分钟执行一次
    loop {
        interval.tick().await;
        data.captcha_manager.cleanup_expired_captchas();
    }
}

// 定期清理过期的黑名单 token
async fn cleanup_expired_tokens(data: web::Data<AppState>) {
    let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(3600)); // 每小时执行一次
    loop {
        interval.tick().await;
        data.auth_manager.cleanup_expired_tokens();
    }
}

// 验证 JWT token
fn validate_token(token: &str) -> Result<Claims, jsonwebtoken::errors::Error> {
    decode::<Claims>(
        token,
        &DecodingKey::from_secret(env::var("JWT_SECRET").unwrap().as_bytes()),
        &Validation::default(),
    )
    .map(|data| data.claims)
}

// VFS API 处理函数
async fn list_directory(
    data: web::Data<AppState>,
    path: web::Path<String>,
    user: web::ReqData<Claims>,
) -> Result<impl Responder> {
    info!("列出目录请求 - 用户: {}, 路径: {}", user.username, path);

    let vfs_user = VfsUser {
        id: user.id.clone(),
        username: user.username.clone(),
        roles: vec![match user.role.as_str() {
            "admin" => Role::Admin,
            "user" => Role::Author,
            _ => Role::Guest,
        }],
    };

    // 确保路径以 / 开头
    let path = if path.starts_with('/') {
        path.to_string()
    } else {
        format!("/{}", path)
    };

    // 设置默认目录为用户目录
    let cwd = format!("/home/{}/", user.username);
    match data.vfs_manager.list_dir(&vfs_user, &path, &cwd).await {
        Ok(nodes) => {
            info!(
                "成功列出目录 - 用户: {}, 路径: {}, 节点数: {}",
                user.username,
                path,
                nodes.len()
            );
            Ok(HttpResponse::Ok().json(nodes))
        }
        Err(e) => {
            error!(
                "列出目录失败 - 用户: {}, 路径: {}, 错误: {:?}",
                user.username, path, e
            );
            Ok(HttpResponse::BadRequest().json(json!({
                "error": format!("{:?}", e)
            })))
        }
    }
}

async fn create_directory(
    data: web::Data<AppState>,
    path: web::Path<String>,
    user: web::ReqData<Claims>,
) -> Result<impl Responder> {
    info!("创建目录请求 - 用户: {}, 路径: {}", user.username, path);

    let vfs_user = VfsUser {
        id: user.id.clone(),
        username: user.username.clone(),
        roles: vec![match user.role.as_str() {
            "admin" => Role::Admin,
            "user" => Role::Author,
            _ => Role::Guest,
        }],
    };

    // 确保路径以 / 开头
    let path = if path.starts_with('/') {
        path.to_string()
    } else {
        format!("/{}", path)
    };

    let cwd = format!("/home/{}/", user.username);
    match data.vfs_manager.create_dir(&vfs_user, &path, &cwd).await {
        Ok(node) => {
            info!(
                "成功创建目录 - 用户: {}, 路径: {}, 节点ID: {}",
                user.username, path, node.id
            );
            Ok(HttpResponse::Ok().json(node))
        }
        Err(e) => {
            error!(
                "创建目录失败 - 用户: {}, 路径: {}, 错误: {:?}",
                user.username, path, e
            );
            Ok(HttpResponse::BadRequest().json(json!({
                "error": format!("{:?}", e)
            })))
        }
    }
}

async fn delete_node(
    data: web::Data<AppState>,
    path: web::Path<String>,
    user: web::ReqData<Claims>,
) -> Result<impl Responder> {
    info!("删除节点请求 - 用户: {}, 路径: {}", user.username, path);

    let vfs_user = VfsUser {
        id: user.id.clone(),
        username: user.username.clone(),
        roles: vec![match user.role.as_str() {
            "admin" => Role::Admin,
            "user" => Role::Author,
            _ => Role::Guest,
        }],
    };

    match data.vfs_manager.delete(&vfs_user, &path, "/").await {
        Ok(_) => {
            info!("成功删除节点 - 用户: {}, 路径: {}", user.username, path);
            Ok(HttpResponse::Ok().json(json!({
                "message": "节点删除成功"
            })))
        }
        Err(e) => {
            error!(
                "删除节点失败 - 用户: {}, 路径: {}, 错误: {:?}",
                user.username, path, e
            );
            Ok(HttpResponse::BadRequest().json(json!({
                "error": format!("{:?}", e)
            })))
        }
    }
}

async fn rename_node(
    data: web::Data<AppState>,
    info: web::Json<(String, String)>, // (old_path, new_path)
    user: web::ReqData<Claims>,
) -> Result<impl Responder> {
    let (old_path, new_path) = info.into_inner();
    info!(
        "重命名节点请求 - 用户: {}, 旧路径: {}, 新路径: {}",
        user.username, old_path, new_path
    );

    let vfs_user = VfsUser {
        id: user.id.clone(),
        username: user.username.clone(),
        roles: vec![match user.role.as_str() {
            "admin" => Role::Admin,
            "user" => Role::Author,
            _ => Role::Guest,
        }],
    };

    match data
        .vfs_manager
        .rename(&vfs_user, &old_path, &new_path, "/")
        .await
    {
        Ok(_) => {
            info!(
                "成功重命名节点 - 用户: {}, 旧路径: {}, 新路径: {}",
                user.username, old_path, new_path
            );
            Ok(HttpResponse::Ok().json(json!({
                "message": "节点重命名成功"
            })))
        }
        Err(e) => {
            error!(
                "重命名节点失败 - 用户: {}, 旧路径: {}, 新路径: {}, 错误: {:?}",
                user.username, old_path, new_path, e
            );
            Ok(HttpResponse::BadRequest().json(json!({
                "error": format!("{:?}", e)
            })))
        }
    }
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
    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await
        .expect("Failed to connect to database");

    info!("数据库连接成功");

    // 创建用户表
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS users (
            id VARCHAR PRIMARY KEY,
            username VARCHAR UNIQUE NOT NULL,
            password_hash VARCHAR NOT NULL,
            email VARCHAR,
            gender VARCHAR CHECK (gender IN ('male', 'female', 'other')),
            birthday DATE,
            role VARCHAR NOT NULL DEFAULT 'user' CHECK (role IN ('admin', 'user')),
            created_at TIMESTAMP WITH TIME ZONE NOT NULL
        )
        "#,
    )
    .execute(&pool)
    .await
    .expect("Failed to create users table");

    // 创建VFS节点表
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS vfs_nodes (
            id BIGSERIAL PRIMARY KEY,
            parent_id BIGINT REFERENCES vfs_nodes(id) ON DELETE CASCADE,
            name TEXT NOT NULL,
            is_dir BOOLEAN NOT NULL,
            owner_id VARCHAR NOT NULL,
            permissions SMALLINT NOT NULL,
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            UNIQUE(parent_id, name)
        )
        "#,
    )
    .execute(&pool)
    .await
    .expect("Failed to create vfs_nodes table");

    // 创建索引
    sqlx::query(
        r#"
        CREATE INDEX IF NOT EXISTS idx_vfs_parent ON vfs_nodes(parent_id)
        "#,
    )
    .execute(&pool)
    .await
    .expect("Failed to create vfs_nodes index");

    // 检查是否需要添加 role 列
    let check_columns = sqlx::query!(
        r#"
        SELECT column_name 
        FROM information_schema.columns 
        WHERE table_name = 'users'
        "#
    )
    .fetch_all(&pool)
    .await
    .expect("Failed to check columns");

    let existing_columns: Vec<String> = check_columns
        .iter()
        .filter_map(|row| row.column_name.clone())
        .collect();

    // 添加缺失的列
    if !existing_columns.contains(&"email".to_string()) {
        sqlx::query("ALTER TABLE users ADD COLUMN email VARCHAR")
            .execute(&pool)
            .await
            .expect("Failed to add email column");
    }

    if !existing_columns.contains(&"gender".to_string()) {
        sqlx::query("ALTER TABLE users ADD COLUMN gender VARCHAR CHECK (gender IN ('male', 'female', 'other'))")
            .execute(&pool)
            .await
            .expect("Failed to add gender column");
    }

    if !existing_columns.contains(&"birthday".to_string()) {
        sqlx::query("ALTER TABLE users ADD COLUMN birthday DATE")
            .execute(&pool)
            .await
            .expect("Failed to add birthday column");
    }

    if !existing_columns.contains(&"role".to_string()) {
        sqlx::query("ALTER TABLE users ADD COLUMN role VARCHAR NOT NULL DEFAULT 'user' CHECK (role IN ('admin', 'user'))")
            .execute(&pool)
            .await
            .expect("Failed to add role column");
    }

    // 初始化VFS后端
    let vfs_backend = PostgresBackend::new(pool.clone());
    vfs_backend.init().await.map_err(|e| {
        error!("VFS初始化失败: {:?}", e);
        std::io::Error::new(std::io::ErrorKind::Other, format!("{:?}", e))
    })?;

    let vfs_manager = VfsManager::new(vfs_backend);

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
        cleanup_expired_captchas(cleanup_state).await;
    });

    // 启动 token 黑名单清理任务
    let token_cleanup_state = app_state.clone();
    tokio::spawn(async move {
        cleanup_expired_tokens(token_cleanup_state).await;
    });

    println!("服务器启动在 http://0.0.0.0:8080");

    // 创建认证中间件
    let auth_middleware = auth::AuthMiddleware::new();

    HttpServer::new(move || {
        App::new()
            .app_data(app_state.clone())
            .wrap(middleware::Logger::default())
            .wrap(
                Cors::default()
                    .allow_any_origin()
                    .allow_any_method()
                    .allow_any_header()
                    .max_age(3600),
            )
            .route("/api/command", web::post().to(handle_command))
            .route("/api/captcha", web::get().to(handle_get_captcha))
            .service(
                web::scope("/api/vfs")
                    .wrap(auth_middleware.clone())
                    .route("/list/{path:.*}", web::get().to(list_directory))
                    .route("/mkdir/{path:.*}", web::post().to(create_directory))
                    .route("/delete/{path:.*}", web::delete().to(delete_node))
                    .route("/rename", web::post().to(rename_node)),
            )
    })
    .bind(("0.0.0.0", 8080))?
    .run()
    .await
}
