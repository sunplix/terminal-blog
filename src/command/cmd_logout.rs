use super::CommandHandler;
use crate::auth::AuthManager;
use actix_web::{web, HttpResponse};
use async_trait::async_trait;
use log::{debug, error, info, warn};

pub struct LogoutCommand;

impl LogoutCommand {
    pub fn new() -> Self {
        LogoutCommand
    }
}

#[async_trait]
impl CommandHandler for LogoutCommand {
    fn name(&self) -> &'static str {
        "logout"
    }

    fn description(&self) -> &'static str {
        "用户登出，用法：logout"
    }

    async fn handle(
        &self,
        _args: &[&str],
        data: &web::Data<crate::AppState>,
        session_id: &str,
    ) -> HttpResponse {
        info!("开始处理登出命令");

        // 从 session_id 中获取 token
        if session_id.is_empty() {
            warn!("未提供 token");
            return HttpResponse::BadRequest().json(super::CommandResponse {
                success: false,
                message: "未提供 token".to_string(),
                data: None,
            });
        }

        debug!("将 token 加入黑名单");
        data.auth_manager.blacklist_token(session_id);

        info!("用户登出成功");
        HttpResponse::Ok().json(super::CommandResponse {
            success: true,
            message: "登出成功".to_string(),
            data: None,
        })
    }
}
