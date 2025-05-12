use super::CommandHandler;
use actix_web::{web, HttpResponse};
use async_trait::async_trait;
use serde_json::Value;

pub struct HelpCommand;

impl HelpCommand {
    pub fn new() -> Self {
        HelpCommand
    }
}

#[async_trait]
impl CommandHandler for HelpCommand {
    fn name(&self) -> &'static str {
        "help"
    }

    fn description(&self) -> &'static str {
        "显示所有可用命令的帮助信息"
    }

    async fn handle(
        &self,
        args: &[&str],
        data: &web::Data<crate::AppState>,
        session_id: &str,
        cwd: &str,
    ) -> HttpResponse {
        let mut commands_info = Vec::new();

        // 获取所有已注册的命令
        for (name, handler) in &data.command_registry.commands {
            commands_info.push(format!("- {}: {}", name, handler.description()));
        }

        // 按命令名称排序
        commands_info.sort();

        // 构建帮助信息
        let help_text = format!("可用命令:\n{}", commands_info.join("\n"));

        HttpResponse::Ok().json(super::CommandResponse {
            success: true,
            message: help_text,
            data: None,
        })
    }
}
