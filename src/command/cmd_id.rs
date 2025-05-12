use super::CommandHandler;
use crate::auth::{validate_token, AuthManager};
use actix_web::{web, HttpResponse};
use async_trait::async_trait;
use log::{debug, info};
use serde_json::json;

pub struct IdCommand;

impl IdCommand {
    pub fn new() -> Self {
        IdCommand
    }
}

#[async_trait]
impl CommandHandler for IdCommand {
    fn name(&self) -> &'static str {
        "id"
    }

    fn description(&self) -> &'static str {
        "显示当前用户信息，用法：id"
    }

    async fn handle(
        &self,
        _args: &[&str],
        data: &web::Data<crate::AppState>,
        token: &str,
    ) -> HttpResponse {
        info!("开始处理ID命令");

        // 验证 token
        if let Ok(claims) = validate_token(token) {
            // 检查 token 是否在黑名单中
            if data.auth_manager.is_token_blacklisted(token) {
                debug!("Token 已失效");
                return HttpResponse::Unauthorized().json(super::CommandResponse {
                    success: false,
                    message: "Token 已失效".to_string(),
                    data: None,
                });
            }

            // 从数据库获取用户信息
            match sqlx::query!(
                "SELECT id, username, role FROM users WHERE id = $1",
                claims.sub
            )
            .fetch_optional(&data.db)
            .await
            {
                Ok(Some(user)) => {
                    debug!("显示用户权限信息: {}", user.username);
                    HttpResponse::Ok().json(super::CommandResponse {
                        success: true,
                        message: format!(
                            "用户ID: {}\n用户名: {}\n角色: {}",
                            user.id, user.username, user.role
                        ),
                        data: Some(json!({
                            "id": user.id,
                            "username": user.username,
                            "role": user.role,
                            "is_guest": false
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
                    debug!("数据库查询错误: {}", e);
                    HttpResponse::InternalServerError().json(super::CommandResponse {
                        success: false,
                        message: "服务器内部错误".to_string(),
                        data: None,
                    })
                }
            }
        } else {
            // 访客模式
            debug!("显示访客信息");
            HttpResponse::Ok().json(super::CommandResponse {
                success: true,
                message: "当前为访客模式".to_string(),
                data: Some(json!({
                    "username": "guest",
                    "role": "guest",
                    "is_guest": true
                })),
            })
        }
    }
}
