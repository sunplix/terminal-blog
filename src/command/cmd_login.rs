use super::CommandHandler;
use crate::auth::{generate_token, AuthManager};
use crate::captcha::CaptchaManager;
use actix_web::{web, HttpResponse};
use async_trait::async_trait;
use log::{debug, error, info, warn};
use serde_json::json;
use sqlx::PgPool;

pub struct LoginCommand;

impl LoginCommand {
    pub fn new() -> Self {
        LoginCommand
    }
}

#[async_trait]
impl CommandHandler for LoginCommand {
    fn name(&self) -> &'static str {
        "login"
    }

    fn description(&self) -> &'static str {
        "用户登录，用法：login <username> <password> [--captcha <code>]"
    }

    async fn handle(
        &self,
        args: &[&str],
        data: &web::Data<crate::AppState>,
        session_id: &str,
    ) -> HttpResponse {
        info!("开始处理登录命令");

        if args.len() < 3 {
            warn!("登录命令参数不足");
            return HttpResponse::BadRequest().json(super::CommandResponse {
                success: false,
                message: "请提供用户名和密码".to_string(),
                data: None,
            });
        }

        let username = args[1];
        let password = args[2];
        debug!("尝试登录用户: {}", username);

        // 检查登录尝试次数
        if let Err(e) = data.auth_manager.check_login_attempts(username) {
            warn!("登录尝试次数过多: {}", e);
            return HttpResponse::TooManyRequests().json(super::CommandResponse {
                success: false,
                message: e,
                data: None,
            });
        }

        // 解析参数
        let mut captcha_code = None;
        let mut i = 3;
        while i < args.len() {
            match args[i] {
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
        }

        // 查找用户
        match sqlx::query!(
            "SELECT id, username, password_hash, role FROM users WHERE username = $1",
            username
        )
        .fetch_optional(&data.db)
        .await
        {
            Ok(Some(user)) => {
                if AuthManager::verify_password(password, &user.password_hash).unwrap_or(false) {
                    info!("用户 {} 登录成功", username);

                    // 生成 JWT token
                    let token = match generate_token(&user.id, &user.username, &user.role) {
                        Ok(token) => token,
                        Err(e) => {
                            error!("生成token失败: {}", e);
                            return HttpResponse::InternalServerError().json(
                                super::CommandResponse {
                                    success: false,
                                    message: "服务器内部错误".to_string(),
                                    data: None,
                                },
                            );
                        }
                    };

                    // 重置登录尝试次数
                    data.auth_manager.reset_login_attempts(username);

                    HttpResponse::Ok().json(super::CommandResponse {
                        success: true,
                        message: "登录成功".to_string(),
                        data: Some(json!({
                            "token": token,
                            "user": {
                                "id": user.id,
                                "username": user.username,
                                "role": user.role
                            }
                        })),
                    })
                } else {
                    // 记录失败的登录尝试
                    data.auth_manager.record_failed_attempt(username);
                    warn!("用户 {} 密码错误", username);
                    HttpResponse::Unauthorized().json(super::CommandResponse {
                        success: false,
                        message: "用户名或密码错误".to_string(),
                        data: None,
                    })
                }
            }
            Ok(None) => {
                // 记录失败的登录尝试
                data.auth_manager.record_failed_attempt(username);
                warn!("用户 {} 不存在", username);
                HttpResponse::Unauthorized().json(super::CommandResponse {
                    success: false,
                    message: "用户名或密码错误".to_string(),
                    data: None,
                })
            }
            Err(e) => {
                error!("数据库查询错误: {}", e);
                HttpResponse::InternalServerError().json(super::CommandResponse {
                    success: false,
                    message: "服务器内部错误".to_string(),
                    data: None,
                })
            }
        }
    }
}
