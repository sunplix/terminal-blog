use actix_web::{web, App, HttpResponse, HttpServer, Responder, http::header, Result};
use actix_files::NamedFile;
use serde::{Deserialize, Serialize};
use std::sync::Mutex;
use std::collections::HashMap;
use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;
use bcrypt::{hash, verify, DEFAULT_COST};
use uuid::Uuid;
use chrono::{DateTime, Utc};
use jsonwebtoken::{encode, decode, Header, EncodingKey, DecodingKey, Validation};
use std::env;
use regex::Regex;
use std::time::{SystemTime, UNIX_EPOCH};
use mime_guess;
use dotenv::dotenv;
use rand::{thread_rng, Rng};
use rand::distributions::Alphanumeric;

// 用户结构体
#[derive(Debug, Serialize, Deserialize, Clone)]
struct User {
    id: String,
    username: String,
    password_hash: String,
    email: Option<String>,
    gender: Option<String>,
    birthday: Option<chrono::NaiveDate>,
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

// JWT Claims
#[derive(Debug, Serialize, Deserialize)]
struct Claims {
    sub: String,  // user id
    exp: usize,   // expiration time
}

// 验证码结构体
#[derive(Debug, Clone)]
struct Captcha {
    code: String,
    created_at: DateTime<Utc>,
    attempts: i32,  // 添加尝试次数计数
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

// 登录失败计数器
struct LoginAttempt {
    count: i32,
    last_attempt: DateTime<Utc>,
}

// 应用状态
struct AppState {
    db: PgPool,
    login_attempts: Mutex<HashMap<String, LoginAttempt>>,
    captchas: Mutex<HashMap<String, Captcha>>,
    token_blacklist: Mutex<HashMap<String, DateTime<Utc>>>, // 添加 token 黑名单
}

// 生成验证码
fn generate_captcha() -> String {
    let mut rng = thread_rng();
    // 生成6位验证码，包含数字和大写字母
    let chars: String = (0..6)
        .map(|_| {
            if rng.gen_bool(0.5) {
                // 生成数字
                rng.gen_range(0..10).to_string()
            } else {
                // 生成大写字母
                (rng.gen_range(b'A'..=b'Z') as char).to_string()
            }
        })
        .collect();
    chars
}

// 命令响应结构体
#[derive(Debug, Serialize)]
struct CommandResponse {
    success: bool,
    message: String,
    data: Option<serde_json::Value>,
}

// 验证 JWT token
fn validate_token(token: &str) -> Result<Claims, jsonwebtoken::errors::Error> {
    decode::<Claims>(
        token,
        &DecodingKey::from_secret(env::var("JWT_SECRET").unwrap().as_bytes()),
        &Validation::default(),
    ).map(|data| data.claims)
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
        },
        Err(_) => Err("生日格式必须是 YYYY-MM-DD".to_string()),
    }
}

// 处理命令的路由
async fn handle_command(
    cmd: web::Json<serde_json::Value>,
    data: web::Data<AppState>,
    req: actix_web::HttpRequest,
) -> impl Responder {
    let command = cmd.get("command").and_then(|v| v.as_str()).unwrap_or("");
    let session_id = cmd.get("session_id").and_then(|v| v.as_str()).unwrap_or("");
    println!("收到命令: {}", command);
    println!("会话ID: {}", session_id);
    
    let args: Vec<&str> = command.split_whitespace().collect();
    if args.is_empty() {
        println!("命令为空");
        return HttpResponse::BadRequest().json(CommandResponse {
            success: false,
            message: "命令不能为空".to_string(),
            data: None,
        });
    }
    
    // 获取当前用户信息
    let current_user = match req.headers().get(header::AUTHORIZATION) {
        Some(token) => {
            println!("发现认证token");
            let token_str = token.to_str().unwrap_or("");
            if token_str.starts_with("Bearer ") {
                println!("验证token");
                let token = &token_str[7..];
                
                // 检查 token 是否在黑名单中
                let blacklist = data.token_blacklist.lock().unwrap();
                if blacklist.contains_key(token) {
                    println!("token 已被加入黑名单");
                    return HttpResponse::Unauthorized().json(CommandResponse {
                        success: false,
                        message: "登录已失效，请重新登录".to_string(),
                        data: None,
                    });
                }
                
                match validate_token(token) {
                    Ok(claims) => {
                        println!("token验证成功，用户ID: {}", claims.sub);
                        // 查询用户信息
                        match sqlx::query!(
                            "SELECT id, username, email, gender, birthday FROM users WHERE id = $1",
                            claims.sub
                        )
                        .fetch_optional(&data.db)
                        .await
                        {
                            Ok(Some(user)) => {
                                println!("找到用户: {}", user.username);
                                Some(user)
                            },
                            _ => {
                                println!("未找到用户");
                                None
                            },
                        }
                    }
                    _ => {
                        println!("token验证失败");
                        None
                    },
                }
            } else {
                println!("无效的token格式");
                None
            }
        }
        None => {
            println!("未提供认证token");
            None
        }
    };

    // 检查命令权限
    match args[0] {
        "login" | "register" => {
            if current_user.is_some() {
                println!("已登录用户尝试使用登录/注册命令");
                return HttpResponse::BadRequest().json(CommandResponse {
                    success: false,
                    message: "您已登录，请先退出登录".to_string(),
                    data: None,
                });
            }
        }
        "logout" => {
            if current_user.is_none() {
                println!("未登录用户尝试使用登出命令");
                return HttpResponse::BadRequest().json(CommandResponse {
                    success: false,
                    message: "您尚未登录".to_string(),
                    data: None,
                });
            }
        }
        _ => {}
    }
    
    match args[0] {
        "help" => {
            println!("显示帮助信息");
            let help_text = r#"可用命令:
- help: 显示帮助信息
- clear: 清除终端输出
- register <username> <password>: 注册新用户
- login <username> <password>: 登录
- logout: 退出登录
- id: 显示当前用户信息
- profile: 更新用户信息"#;
            HttpResponse::Ok().json(CommandResponse {
                success: true,
                message: help_text.to_string(),
                data: None,
            })
        }
        "register" => {
            println!("处理注册命令");
            if args.len() < 3 {
                println!("参数不足");
                return HttpResponse::BadRequest().json(CommandResponse {
                    success: false,
                    message: "用法: register <username> <password> [--confirm <password>] [--captcha <code>]".to_string(),
                    data: None,
                });
            }
            
            let username = args[1];
            let password = args[2];
            println!("注册用户: {}", username);
            
            // 检查是否需要确认密码
            let confirm_index = args.iter().position(|&x| x == "--confirm");
            if let Some(index) = confirm_index {
                if index + 1 >= args.len() {
                    println!("缺少确认密码");
                    return HttpResponse::BadRequest().json(CommandResponse {
                        success: false,
                        message: "请提供确认密码".to_string(),
                        data: None,
                    });
                }
                let confirm_password = args[index + 1];
                if password != confirm_password {
                    println!("密码不匹配");
                    return HttpResponse::BadRequest().json(CommandResponse {
                        success: false,
                        message: "两次输入的密码不一致".to_string(),
                        data: None,
                    });
                }
            }

            // 验证验证码
            let captcha_index = args.iter().position(|&x| x == "--captcha");
            if let Some(index) = captcha_index {
                if index + 1 >= args.len() {
                    println!("缺少验证码");
                    return HttpResponse::BadRequest().json(CommandResponse {
                        success: false,
                        message: "请提供验证码".to_string(),
                        data: None,
                    });
                }
                let captcha_code = args[index + 1];
                println!("验证验证码: {}", captcha_code);
                if !verify_captcha(&data, session_id, captcha_code) {
                    println!("验证码错误或已过期");
                    return HttpResponse::BadRequest().json(CommandResponse {
                        success: false,
                        message: "验证码错误或已过期".to_string(),
                        data: None,
                    });
                }
            } else {
                println!("缺少验证码");
                return HttpResponse::BadRequest().json(CommandResponse {
                    success: false,
                    message: "注册时需要提供验证码".to_string(),
                    data: None,
                });
            }
            
            // 验证用户名
            if let Err(e) = validate_username(username) {
                println!("用户名验证失败: {}", e);
                return HttpResponse::BadRequest().json(CommandResponse {
                    success: false,
                    message: e,
                    data: None,
                });
            }
            
            // 验证密码强度
            if let Err(e) = validate_password(password) {
                println!("密码验证失败: {}", e);
                return HttpResponse::BadRequest().json(CommandResponse {
                    success: false,
                    message: e,
                    data: None,
                });
            }
            
            // 检查用户名是否已存在
            let existing_user = sqlx::query!(
                "SELECT id FROM users WHERE username = $1",
                username
            )
            .fetch_optional(&data.db)
            .await
            .unwrap();
            
            if existing_user.is_some() {
                println!("用户名已存在");
                return HttpResponse::BadRequest().json(CommandResponse {
                    success: false,
                    message: "用户名已存在".to_string(),
                    data: None,
                });
            }
            
            // 创建新用户
            let user_id = Uuid::new_v4().to_string();
            let password_hash = hash(password.as_bytes(), DEFAULT_COST).unwrap();
            
            sqlx::query!(
                "INSERT INTO users (id, username, password_hash, created_at) VALUES ($1, $2, $3, NOW())",
                user_id,
                username,
                password_hash
            )
            .execute(&data.db)
            .await
            .unwrap();
            
            println!("用户注册成功");
            HttpResponse::Ok().json(CommandResponse {
                success: true,
                message: "注册成功".to_string(),
                data: None,
            })
        }
        "login" => {
            println!("处理登录命令");
            if args.len() < 3 {
                println!("参数数量错误");
                return HttpResponse::BadRequest().json(CommandResponse {
                    success: false,
                    message: "用法: login <username> <password> [--captcha <code>]".to_string(),
                    data: None,
                });
            }
            
            let username = args[1];
            let password = args[2];
            println!("尝试登录用户: {}", username);
            
            // 检查登录尝试次数
            let mut attempts = data.login_attempts.lock().unwrap();
            let attempt = attempts.entry(username.to_string()).or_insert(LoginAttempt {
                count: 0,
                last_attempt: Utc::now(),
            });
            
            // 如果失败次数过多，检查是否需要等待
            if attempt.count >= 5 {
                let wait_time = chrono::Duration::minutes(15);
                if Utc::now() - attempt.last_attempt < wait_time {
                    println!("登录尝试次数过多");
                    return HttpResponse::TooManyRequests().json(CommandResponse {
                        success: false,
                        message: format!(
                            "登录尝试次数过多，请等待 {} 分钟后再试",
                            (wait_time - (Utc::now() - attempt.last_attempt)).num_minutes()
                        ),
                        data: None,
                    });
                }
                // 重置计数器
                attempt.count = 0;
            }

            // 验证验证码
            let captcha_index = args.iter().position(|&x| x == "--captcha");
            if let Some(index) = captcha_index {
                if index + 1 >= args.len() {
                    println!("缺少验证码");
                    return HttpResponse::BadRequest().json(CommandResponse {
                        success: false,
                        message: "请提供验证码".to_string(),
                        data: None,
                    });
                }
                let captcha_code = args[index + 1];
                println!("验证验证码: {}", captcha_code);
                if !verify_captcha(&data, session_id, captcha_code) {
                    println!("验证码错误或已过期");
                    return HttpResponse::BadRequest().json(CommandResponse {
                        success: false,
                        message: "验证码错误或已过期".to_string(),
                        data: None,
                    });
                }
            } else if attempt.count >= 3 {
                // 如果登录失败次数达到3次，强制要求验证码
                println!("登录失败次数过多，需要验证码");
                return HttpResponse::BadRequest().json(CommandResponse {
                    success: false,
                    message: "登录失败次数过多，请使用验证码重新登录".to_string(),
                    data: None,
                });
            }
            
            // 查找用户
            let user = sqlx::query!(
                "SELECT id, password_hash FROM users WHERE username = $1",
                username
            )
            .fetch_optional(&data.db)
            .await
            .unwrap();
            
            match user {
                Some(user) => {
                    if verify(password, &user.password_hash).unwrap() {
                        // 登录成功，重置计数器
                        attempt.count = 0;
                        println!("密码验证成功");
                        
                        // 生成 JWT token
                        let exp = chrono::Utc::now()
                            .checked_add_signed(chrono::Duration::hours(24))
                            .unwrap()
                            .timestamp() as usize;
                        
                        let claims = Claims {
                            sub: user.id,
                            exp,
                        };
                        
                        let token = encode(
                            &Header::default(),
                            &claims,
                            &EncodingKey::from_secret(env::var("JWT_SECRET").unwrap().as_bytes())
                        ).unwrap();
                        
                        println!("生成token成功");
                        HttpResponse::Ok().json(CommandResponse {
                            success: true,
                            message: "登录成功".to_string(),
                            data: Some(serde_json::json!({
                                "token": token
                            })),
                        })
                    } else {
                        // 登录失败，增加计数器
                        attempt.count += 1;
                        attempt.last_attempt = Utc::now();
                        println!("密码错误");
                        
                        HttpResponse::BadRequest().json(CommandResponse {
                            success: false,
                            message: "密码错误".to_string(),
                            data: None,
                        })
                    }
                }
                None => {
                    // 用户不存在，增加计数器
                    attempt.count += 1;
                    attempt.last_attempt = Utc::now();
                    println!("用户不存在");
                    
                    HttpResponse::BadRequest().json(CommandResponse {
                        success: false,
                        message: "用户不存在".to_string(),
                        data: None,
                    })
                }
            }
        }
        "logout" => {
            println!("处理登出命令");
            if let Some(token) = req.headers().get(header::AUTHORIZATION) {
                if let Ok(token_str) = token.to_str() {
                    if token_str.starts_with("Bearer ") {
                        let token = &token_str[7..];
                        // 将 token 加入黑名单
                        let mut blacklist = data.token_blacklist.lock().unwrap();
                        blacklist.insert(token.to_string(), Utc::now());
                    }
                }
            }
            HttpResponse::Ok().json(CommandResponse {
                success: true,
                message: "已退出登录".to_string(),
                data: None,
            })
        }
        "clear" => {
            println!("处理清除命令");
            HttpResponse::Ok().json(CommandResponse {
                success: true,
                message: "".to_string(),
                data: None,
            })
        }
        "id" => {
            println!("处理ID命令");
            match current_user {
                Some(user) => {
                    println!("显示用户信息: {}", user.username);
                    HttpResponse::Ok().json(CommandResponse {
                        success: true,
                        message: "获取用户信息成功".to_string(),
                        data: Some(serde_json::json!({
                            "id": user.id,
                            "username": user.username,
                            "email": user.email,
                            "gender": user.gender,
                            "birthday": user.birthday.map(|d| d.format("%Y-%m-%d").to_string()),
                            "is_guest": false
                        })),
                    })
                }
                None => {
                    println!("显示访客信息");
                    HttpResponse::Ok().json(CommandResponse {
                        success: true,
                        message: "当前为访客模式".to_string(),
                        data: Some(serde_json::json!({
                            "username": "guest",
                            "is_guest": true
                        })),
                    })
                }
            }
        }
        "profile" => {
            println!("处理个人信息更新命令");
            if current_user.is_none() {
                return HttpResponse::BadRequest().json(CommandResponse {
                        success: false,
                    message: "请先登录".to_string(),
                        data: None,
                    });
                }

            if args.len() < 2 {
                return HttpResponse::BadRequest().json(CommandResponse {
                        success: false,
                    message: "用法: profile --email <email> [--gender <gender>] [--birthday <YYYY-MM-DD>]".to_string(),
                        data: None,
                    });
                }

            let mut email = None;
            let mut gender = None;
            let mut birthday = None;

            // 解析参数
            let mut i = 1;
            while i < args.len() {
                match args[i] {
                    "--email" => {
                        if i + 1 >= args.len() {
                            return HttpResponse::BadRequest().json(CommandResponse {
                                success: false,
                                message: "请提供邮箱地址".to_string(),
                                data: None,
                            });
                        }
                        if let Err(e) = validate_email(args[i + 1]) {
                            return HttpResponse::BadRequest().json(CommandResponse {
                                success: false,
                                message: e,
                                data: None,
                            });
                        }
                        email = Some(args[i + 1].to_string());
                        i += 2;
                    }
                    "--gender" => {
                        if i + 1 >= args.len() {
                            return HttpResponse::BadRequest().json(CommandResponse {
                                success: false,
                                message: "请提供性别".to_string(),
                                data: None,
                            });
                        }
                        if let Err(e) = validate_gender(args[i + 1]) {
                            return HttpResponse::BadRequest().json(CommandResponse {
                                success: false,
                                message: e,
                                data: None,
                            });
                        }
                        gender = Some(args[i + 1].to_string());
                        i += 2;
                    }
                    "--birthday" => {
                        if i + 1 >= args.len() {
                            return HttpResponse::BadRequest().json(CommandResponse {
                                success: false,
                                message: "请提供生日".to_string(),
                                data: None,
                            });
                        }
                        match validate_birthday(args[i + 1]) {
                            Ok(date) => birthday = Some(date),
                            Err(e) => {
                                return HttpResponse::BadRequest().json(CommandResponse {
                        success: false,
                                    message: e,
                        data: None,
                    });
                }
                        }
                        i += 2;
                    }
                    _ => {
                        return HttpResponse::BadRequest().json(CommandResponse {
                        success: false,
                            message: format!("未知参数: {}", args[i]),
                        data: None,
                    });
                }
                }
            }

            // 更新用户信息
            let user_id = current_user.unwrap().id;
            sqlx::query!(
                r#"
                UPDATE users 
                SET email = COALESCE($1, email),
                    gender = COALESCE($2, gender),
                    birthday = COALESCE($3, birthday)
                WHERE id = $4
                "#,
                email,
                gender,
                birthday,
                user_id
            )
            .execute(&data.db)
            .await
            .unwrap();

            HttpResponse::Ok().json(CommandResponse {
                success: true,
                message: "个人信息更新成功".to_string(),
                data: None,
            })
        }
        _ => {
            println!("未知命令: {}", args[0]);
            HttpResponse::BadRequest().json(CommandResponse {
            success: false,
            message: "未知命令".to_string(),
            data: None,
            })
        }
    }
}

// 获取验证码
async fn get_captcha(data: web::Data<AppState>) -> impl Responder {
    let captcha = generate_captcha();
    let session_id = Uuid::new_v4().to_string();
    
    let mut captchas = data.captchas.lock().unwrap();
    captchas.insert(session_id.clone(), Captcha {
        code: captcha.clone(),
        created_at: Utc::now(),
        attempts: 0,
    });

    HttpResponse::Ok().json(serde_json::json!({
        "success": true,
        "data": {
            "session_id": session_id,
            "captcha": captcha
        }
    }))
}

// 验证验证码
fn verify_captcha(data: &web::Data<AppState>, session_id: &str, code: &str) -> bool {
    println!("验证验证码 - 会话ID: {}, 验证码: {}", session_id, code);
    let mut captchas = data.captchas.lock().unwrap();
    if let Some(captcha) = captchas.get_mut(session_id) {
        // 验证码5分钟有效
        if Utc::now() - captcha.created_at > chrono::Duration::minutes(5) {
            println!("验证码已过期");
            captchas.remove(session_id);
            return false;
        }
        
        // 增加尝试次数
        captcha.attempts += 1;
        
        // 如果尝试次数超过3次，验证码失效
        if captcha.attempts > 3 {
            println!("验证码尝试次数过多");
            captchas.remove(session_id);
            return false;
        }
        
        if captcha.code == code {
            println!("验证码正确");
            captchas.remove(session_id);
            return true;
        }
        println!("验证码错误");
    } else {
        println!("未找到验证码会话");
    }
    false
}

// 定期清理过期验证码
async fn cleanup_expired_captchas(data: web::Data<AppState>) {
    let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(600)); // 每10分钟执行一次
    loop {
        interval.tick().await;
        let mut captchas = data.captchas.lock().unwrap();
        captchas.retain(|_, captcha| {
            Utc::now() - captcha.created_at <= chrono::Duration::minutes(5)
        });
    }
}

// 定期清理过期的黑名单 token
async fn cleanup_expired_tokens(data: web::Data<AppState>) {
    let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(3600)); // 每小时执行一次
    loop {
        interval.tick().await;
        let mut blacklist = data.token_blacklist.lock().unwrap();
        blacklist.retain(|_, timestamp| {
            Utc::now() - *timestamp <= chrono::Duration::hours(24)
        });
    }
}

// 处理根路径
async fn index() -> impl Responder {
    let start = SystemTime::now();
    let since_the_epoch = start.duration_since(UNIX_EPOCH).unwrap();
    let timestamp = since_the_epoch.as_secs();

    let html = include_str!("../templates/index.html")
        .replace("{{ timestamp }}", &timestamp.to_string());
    
    HttpResponse::Ok()
        .content_type("text/html; charset=utf-8")
        .insert_header((header::CACHE_CONTROL, "no-store, no-cache, must-revalidate, max-age=0"))
        .insert_header((header::PRAGMA, "no-cache"))
        .insert_header((header::EXPIRES, "0"))
        .body(html)
}

/// 自定义静态文件路由，带 no-cache 头
async fn serve_static(path: web::Path<String>) -> Result<impl Responder> {
    let file_path = format!("static/{}", path);

    // 先打开文件并连同设置 Content-Type 一起消费掉旧的 file，返回新的 NamedFile
    let file = NamedFile::open(&file_path)?
        .set_content_type(mime_guess::from_path(&file_path).first_or_octet_stream());

    // customize 会消费 file，返回一个 CustomizeResponder<NamedFile>
    let response = file
        .customize()
        .insert_header((header::CACHE_CONTROL, "no-store, no-cache, must-revalidate, max-age=0"))
        .insert_header((header::PRAGMA, "no-cache"))
        .insert_header((header::EXPIRES, "0"));

    Ok(response)
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // 加载 .env
    dotenv().ok();
    // 初始化日志
    env_logger::init_from_env(env_logger::Env::new().default_filter_or("info"));

    // 连接数据库
    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await
        .expect("Failed to connect to database");

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
            created_at TIMESTAMP WITH TIME ZONE NOT NULL
        )
        "#,
    )
    .execute(&pool)
    .await
    .expect("Failed to create users table");

    // 检查是否需要添加新列
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

    let existing_columns: Vec<String> = check_columns.iter()
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

    // 应用状态
    let app_state = web::Data::new(AppState {
        db: pool,
        login_attempts: Mutex::new(HashMap::new()),
        captchas: Mutex::new(HashMap::new()),
        token_blacklist: Mutex::new(HashMap::new()),
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

    // 启动 HTTP 服务
    HttpServer::new(move || {
        App::new()
            .app_data(app_state.clone())
            .route("/", web::get().to(index))
            .route("/api/command", web::post().to(handle_command))
            .route("/api/captcha", web::get().to(get_captcha))
            .route("/static/{path:.*}", web::get().to(serve_static))
    })
    .bind(("0.0.0.0", 8080))?
    .run()
    .await
}