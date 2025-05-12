use super::CommandHandler;
use crate::auth::validate_token;
use actix_web::{web, HttpResponse};
use async_trait::async_trait;
use chrono::{NaiveDate, Utc};
use log::{debug, error, info, warn};
use regex::Regex;
use serde_json::json;

pub struct ProfileCommand;

impl ProfileCommand {
    pub fn new() -> Self {
        ProfileCommand
    }
}

#[async_trait]
impl CommandHandler for ProfileCommand {
    fn name(&self) -> &'static str {
        "profile"
    }

    fn description(&self) -> &'static str {
        "显示或更新用户信息，用法：profile show | profile update [--email <email>] [--gender <gender>] [--birthday <YYYY-MM-DD>]"
    }

    async fn handle(
        &self,
        args: &[&str],
        data: &web::Data<crate::AppState>,
        session_id: &str,
        cwd: &str,
    ) -> HttpResponse {
        info!("开始处理 profile 命令");

        // 验证 token
        let claims = match validate_token(session_id) {
            Ok(claims) => claims,
            Err(_) => {
                warn!("未登录");
                return HttpResponse::BadRequest().json(super::CommandResponse {
                    success: false,
                    message: "请先登录".to_string(),
                    data: None,
                });
            }
        };

        // 检查 token 是否在黑名单中
        if data.auth_manager.is_token_blacklisted(session_id) {
            debug!("Token 已失效");
            return HttpResponse::Unauthorized().json(super::CommandResponse {
                success: false,
                message: "Token 已失效".to_string(),
                data: None,
            });
        }

        // 支持 profile show 和 profile update
        if args.len() == 1 || (args.len() == 2 && args[1] == "show") {
            // 显示当前用户信息
            match sqlx::query!(
                r#"
                SELECT id, username, email, gender, birthday, role, created_at
                FROM users
                WHERE id = $1
                "#,
                claims.sub
            )
            .fetch_optional(&data.db)
            .await
            {
                Ok(Some(user)) => {
                    debug!("显示用户信息: {}", user.username);
                    HttpResponse::Ok().json(super::CommandResponse {
                        success: true,
                        message: format!(
                            "用户ID: {}\n用户名: {}\n邮箱: {}\n性别: {}\n生日: {}\n角色: {}\n创建时间: {}",
                            user.id,
                            user.username,
                            user.email.as_ref().unwrap_or(&"未设置".to_string()),
                            user.gender.as_ref().unwrap_or(&"未设置".to_string()),
                            user.birthday.map(|d| d.format("%Y-%m-%d").to_string()).unwrap_or_else(|| "未设置".to_string()),
                            user.role,
                            user.created_at.format("%Y-%m-%d %H:%M:%S").to_string()
                        ),
                        data: Some(json!({
                            "id": user.id,
                            "username": user.username,
                            "email": user.email,
                            "gender": user.gender,
                            "birthday": user.birthday.map(|d| d.format("%Y-%m-%d").to_string()),
                            "role": user.role,
                            "created_at": user.created_at.format("%Y-%m-%d %H:%M:%S").to_string()
                        })),
                    })
                }
                Ok(None) => {
                    debug!("用户不存在");
                    HttpResponse::Unauthorized().json(super::CommandResponse {
                        success: false,
                        message: "用户不存在".to_string(),
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
        } else if args.len() >= 2 && args[1] == "update" {
            let mut email = None;
            let mut gender = None;
            let mut birthday = None;

            // 解析参数
            let mut i = 2;
            while i < args.len() {
                match args[i] {
                    "--email" => {
                        if i + 1 >= args.len() {
                            return HttpResponse::BadRequest().json(super::CommandResponse {
                                success: false,
                                message: "请提供邮箱地址".to_string(),
                                data: None,
                            });
                        }
                        // 简单的邮箱格式验证
                        let email_str = args[i + 1];
                        let email_regex =
                            Regex::new(r"^[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}$")
                                .unwrap();
                        if !email_regex.is_match(email_str) {
                            return HttpResponse::BadRequest().json(super::CommandResponse {
                                success: false,
                                message: "邮箱格式不正确".to_string(),
                                data: None,
                            });
                        }
                        email = Some(email_str.to_string());
                        i += 2;
                    }
                    "--gender" => {
                        if i + 1 >= args.len() {
                            return HttpResponse::BadRequest().json(super::CommandResponse {
                                success: false,
                                message: "请提供性别".to_string(),
                                data: None,
                            });
                        }
                        match args[i + 1] {
                            "male" | "female" | "other" => {
                                gender = Some(args[i + 1].to_string());
                            }
                            _ => {
                                return HttpResponse::BadRequest().json(super::CommandResponse {
                                    success: false,
                                    message: "性别必须是 male、female 或 other".to_string(),
                                    data: None,
                                });
                            }
                        }
                        i += 2;
                    }
                    "--birthday" => {
                        if i + 1 >= args.len() {
                            return HttpResponse::BadRequest().json(super::CommandResponse {
                                success: false,
                                message: "请提供生日".to_string(),
                                data: None,
                            });
                        }
                        match NaiveDate::parse_from_str(args[i + 1], "%Y-%m-%d") {
                            Ok(date) => {
                                if date > Utc::now().date_naive() {
                                    return HttpResponse::BadRequest().json(
                                        super::CommandResponse {
                                            success: false,
                                            message: "生日不能是未来日期".to_string(),
                                            data: None,
                                        },
                                    );
                                }
                                birthday = Some(date);
                            }
                            Err(_) => {
                                return HttpResponse::BadRequest().json(super::CommandResponse {
                                    success: false,
                                    message: "生日格式不正确，请使用 YYYY-MM-DD 格式".to_string(),
                                    data: None,
                                });
                            }
                        }
                        i += 2;
                    }
                    _ => {
                        return HttpResponse::BadRequest().json(super::CommandResponse {
                            success: false,
                            message: format!("未知参数: {}", args[i]),
                            data: None,
                        });
                    }
                }
            }

            // 更新用户信息
            match sqlx::query!(
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
                claims.sub
            )
            .execute(&data.db)
            .await
            {
                Ok(_) => {
                    info!("用户信息更新成功");
                    HttpResponse::Ok().json(super::CommandResponse {
                        success: true,
                        message: "个人信息更新成功".to_string(),
                        data: None,
                    })
                }
                Err(e) => {
                    error!("更新用户信息失败: {}", e);
                    HttpResponse::InternalServerError().json(super::CommandResponse {
                        success: false,
                        message: "服务器内部错误".to_string(),
                        data: None,
                    })
                }
            }
        } else {
            // 其他情况，返回用法
            HttpResponse::BadRequest().json(super::CommandResponse {
                success: false,
                message: "用法: profile show | profile update [--email <email>] [--gender <gender>] [--birthday <YYYY-MM-DD>]".to_string(),
                data: None,
            })
        }
    }
}
