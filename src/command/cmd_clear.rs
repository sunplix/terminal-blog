use super::CommandHandler;
use actix_web::{web, HttpResponse};
use async_trait::async_trait;
use log::{debug, info};

pub struct ClearCommand;

impl ClearCommand {
    pub fn new() -> Self {
        ClearCommand
    }
}

#[async_trait]
impl CommandHandler for ClearCommand {
    fn name(&self) -> &'static str {
        "clear"
    }

    fn description(&self) -> &'static str {
        "清除屏幕，用法：clear"
    }

    async fn handle(
        &self,
        args: &[&str],
        data: &web::Data<crate::AppState>,
        session_id: &str,
        cwd: &str,
    ) -> HttpResponse {
        info!("开始处理清除命令");
        debug!("清除屏幕");

        HttpResponse::Ok().json(super::CommandResponse {
            success: true,
            message: "".to_string(),
            data: None,
        })
    }
}
