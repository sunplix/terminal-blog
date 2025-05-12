use super::CommandHandler;
use crate::auth::validate_token;
use actix_web::{web, HttpResponse};
use async_trait::async_trait;
use log::{debug, info};

pub struct PwdCommand;

impl PwdCommand {
    pub fn new() -> Self {
        PwdCommand
    }
}

#[async_trait]
impl CommandHandler for PwdCommand {
    fn name(&self) -> &'static str {
        "pwd"
    }

    fn description(&self) -> &'static str {
        "显示当前工作目录，用法：pwd"
    }

    async fn handle(
        &self,
        _args: &[&str],
        data: &web::Data<crate::AppState>,
        session_id: &str,
        cwd: &str,
    ) -> HttpResponse {
        info!("开始处理 pwd 命令");

        // 获取当前用户名（如果已登录）
        let path = match validate_token(session_id) {
            Ok(claims) => {
                // 从数据库获取用户名
                match sqlx::query!("SELECT username FROM users WHERE id = $1", claims.sub)
                    .fetch_optional(&data.db)
                    .await
                {
                    Ok(Some(user)) => {
                        debug!("用户已登录: {}", user.username);
                        format!("/home/{}/", user.username)
                    }
                    _ => {
                        debug!("用户不存在");
                        "/home/guest/".to_string()
                    }
                }
            }
            Err(_) => {
                debug!("用户未登录，使用访客目录");
                "/home/guest/".to_string()
            }
        };

        HttpResponse::Ok().json(super::CommandResponse {
            success: true,
            message: path,
            data: None,
        })
    }
}
