use super::CommandHandler;
use crate::auth::AuthManager;
use crate::captcha::CaptchaManager;
use crate::vfs::model::{Role, User as VfsUser};
use actix_web::{web, HttpResponse};
use chrono::Utc;
use log::{debug, error, info, warn};
use sqlx::PgPool;
use uuid::Uuid;

pub struct RegisterCommand;

impl RegisterCommand {
    pub fn new() -> Self {
        RegisterCommand
    }
}

#[async_trait::async_trait]
impl CommandHandler for RegisterCommand {
    fn name(&self) -> &'static str {
        "register"
    }

    fn description(&self) -> &'static str {
        "注册新用户，用法：register <username> <password> --confirm <password> --captcha <code> [--show]"
    }

    async fn handle(
        &self,
        args: &[&str],
        data: &web::Data<crate::AppState>,
        session_id: &str,
    ) -> HttpResponse {
        info!("开始处理注册命令");

        if args.len() < 3 {
            warn!("注册命令参数不足");
            return HttpResponse::BadRequest().json(super::CommandResponse {
                success: false,
                message: "请提供用户名和密码".to_string(),
                data: None,
            });
        }

        let username = args[1];
        let password = args[2];
        debug!("注册用户: {}", username);

        // 解析参数
        let mut confirm_password = None;
        let mut captcha_code = None;
        let mut show_password = false;

        let mut i = 3;
        while i < args.len() {
            match args[i] {
                "--confirm" => {
                    if i + 1 < args.len() {
                        confirm_password = Some(args[i + 1]);
                        i += 2;
                    } else {
                        warn!("缺少确认密码");
                        return HttpResponse::BadRequest().json(super::CommandResponse {
                            success: false,
                            message: "请提供确认密码".to_string(),
                            data: None,
                        });
                    }
                }
                "--captcha" => {
                    if i + 1 < args.len() {
                        captcha_code = Some(args[i + 1]);
                        i += 2;
                    } else {
                        warn!("缺少验证码");
                        return HttpResponse::BadRequest().json(super::CommandResponse {
                            success: false,
                            message: "请提供验证码".to_string(),
                            data: None,
                        });
                    }
                }
                "--show" => {
                    show_password = true;
                    i += 1;
                }
                _ => {
                    warn!("未知参数: {}", args[i]);
                    return HttpResponse::BadRequest().json(super::CommandResponse {
                        success: false,
                        message: format!("未知参数: {}", args[i]),
                        data: None,
                    });
                }
            }
        }

        // 验证密码确认
        if let Some(confirm) = confirm_password {
            if confirm != password {
                warn!("密码不匹配");
                return HttpResponse::BadRequest().json(super::CommandResponse {
                    success: false,
                    message: "两次输入的密码不一致".to_string(),
                    data: None,
                });
            }
        } else {
            warn!("缺少密码确认");
            return HttpResponse::BadRequest().json(super::CommandResponse {
                success: false,
                message: "请使用 --confirm 参数确认密码".to_string(),
                data: None,
            });
        }

        // 验证验证码
        if let Some(code) = captcha_code {
            debug!("验证验证码: {}", code);
            if !CaptchaManager::verify_captcha(&data.captcha_manager, session_id, code) {
                warn!("验证码错误");
                return HttpResponse::BadRequest().json(super::CommandResponse {
                    success: false,
                    message: "验证码错误".to_string(),
                    data: None,
                });
            }
        } else {
            warn!("缺少验证码");
            return HttpResponse::BadRequest().json(super::CommandResponse {
                success: false,
                message: "请提供验证码".to_string(),
                data: None,
            });
        }

        // 验证用户名
        if let Err(e) = validate_username(username) {
            warn!("用户名验证失败: {}", e);
            return HttpResponse::BadRequest().json(super::CommandResponse {
                success: false,
                message: e,
                data: None,
            });
        }

        // 验证密码
        if let Err(e) = validate_password(password) {
            warn!("密码验证失败: {}", e);
            return HttpResponse::BadRequest().json(super::CommandResponse {
                success: false,
                message: e,
                data: None,
            });
        }

        // 使用 actix_web::web::block 来执行阻塞操作
        let db = data.db.clone();
        let username = username.to_string();
        let password = password.to_string();

        // 检查用户名是否已存在
        match sqlx::query!("SELECT id FROM users WHERE username = $1", username)
            .fetch_optional(&db)
            .await
        {
            Ok(Some(_)) => {
                warn!("用户名已存在: {}", username);
                return HttpResponse::BadRequest().json(super::CommandResponse {
                    success: false,
                    message: "用户名已存在".to_string(),
                    data: None,
                });
            }
            Ok(None) => {
                debug!("用户名可用: {}", username);
            }
            Err(e) => {
                error!("数据库查询错误: {}", e);
                return HttpResponse::InternalServerError().json(super::CommandResponse {
                    success: false,
                    message: "服务器内部错误".to_string(),
                    data: None,
                });
            }
        }

        // 加密密码
        let password_hash = match AuthManager::hash_password(&password) {
            Ok(hash) => hash,
            Err(e) => {
                error!("密码加密失败: {}", e);
                return HttpResponse::InternalServerError().json(super::CommandResponse {
                    success: false,
                    message: "服务器内部错误".to_string(),
                    data: None,
                });
            }
        };

        // 创建用户
        let user_id = Uuid::new_v4().to_string();
        match sqlx::query!(
            "INSERT INTO users (id, username, password_hash, role, created_at) VALUES ($1, $2, $3, $4, $5)",
            user_id,
            username,
            password_hash,
            "user",
            Utc::now()
        )
        .execute(&db)
        .await
        {
            Ok(_) => {
                info!("用户注册成功: {}", username);
                
                // 创建用户目录
                let user_dir = format!("/home/{}", username);
                let system_admin = VfsUser {
                    id: "system".to_string(),
                    username: "system".to_string(),
                    roles: vec![Role::Admin],
                };
                if let Err(e) = data.vfs_manager.create_dir(&system_admin, &user_dir, "/").await {
                    error!("创建用户目录失败: {}", e);
                    return HttpResponse::InternalServerError().json(super::CommandResponse {
                        success: false,
                        message: "创建用户目录失败".to_string(),
                        data: None,
                    });
                }
                
                // 更新目录所有者为新用户
                if let Err(e) = data.vfs_manager.update_node_owner(&user_dir, &user_id).await {
                    error!("更新目录所有者失败: {}", e);
                    return HttpResponse::InternalServerError().json(super::CommandResponse {
                        success: false,
                        message: "更新目录所有者失败".to_string(),
                        data: None,
                    });
                }
                
                HttpResponse::Ok().json(super::CommandResponse {
                    success: true,
                    message: "注册成功".to_string(),
                    data: None,
                })
            }
            Err(e) => {
                error!("用户注册失败: {}", e);
                HttpResponse::InternalServerError().json(super::CommandResponse {
                    success: false,
                    message: "服务器内部错误".to_string(),
                    data: None,
                })
            }
        }
    }
}

// 用户名验证函数
fn validate_username(username: &str) -> Result<(), String> {
    if username.len() < 3 {
        return Err("用户名长度必须大于等于3个字符".to_string());
    }
    if username.len() > 20 {
        return Err("用户名长度不能超过20个字符".to_string());
    }
    if !username.chars().all(|c| c.is_alphanumeric() || c == '_') {
        return Err("用户名只能包含字母、数字和下划线".to_string());
    }
    Ok(())
}

// 密码验证函数
fn validate_password(password: &str) -> Result<(), String> {
    if password.len() < 6 {
        return Err("密码长度必须大于等于6个字符".to_string());
    }
    if password.len() > 32 {
        return Err("密码长度不能超过32个字符".to_string());
    }
    Ok(())
}
