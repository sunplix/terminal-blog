use actix_web::{web, HttpResponse, Responder};
use async_trait::async_trait;
use log::{debug, error, info, warn};
use serde_json::Value;
use std::collections::HashMap;

mod cmd_clear;
mod cmd_help;
mod cmd_id;
mod cmd_login;
mod cmd_logout;
mod cmd_ls;
mod cmd_mkdir;
mod cmd_profile;
mod cmd_pwd;
mod cmd_register;

// 命令处理器的trait
#[async_trait]
pub trait CommandHandler: Send + Sync {
    fn name(&self) -> &'static str;
    fn description(&self) -> &'static str;
    async fn handle(
        &self,
        args: &[&str],
        data: &web::Data<crate::AppState>,
        session_id: &str,
    ) -> HttpResponse;
}

// 命令注册器
pub struct CommandRegistry {
    commands: HashMap<String, Box<dyn CommandHandler>>,
}

impl CommandRegistry {
    pub fn new() -> Self {
        let mut registry = CommandRegistry {
            commands: HashMap::new(),
        };

        // 注册所有命令
        registry.register(Box::new(cmd_help::HelpCommand::new()));
        registry.register(Box::new(cmd_login::LoginCommand::new()));
        registry.register(Box::new(cmd_register::RegisterCommand::new()));
        registry.register(Box::new(cmd_logout::LogoutCommand::new()));
        registry.register(Box::new(cmd_clear::ClearCommand::new()));
        registry.register(Box::new(cmd_id::IdCommand::new()));
        registry.register(Box::new(cmd_profile::ProfileCommand::new()));
        registry.register(Box::new(cmd_ls::LsCommand::new()));
        registry.register(Box::new(cmd_pwd::PwdCommand::new()));
        registry.register(Box::new(cmd_mkdir::MkdirCommand::new()));

        info!("命令注册器初始化完成");
        registry
    }

    pub fn register(&mut self, handler: Box<dyn CommandHandler>) {
        let name = handler.name().to_string();
        self.commands.insert(name.clone(), handler);
        debug!("注册命令: {}", name);
    }

    pub fn get_handler(&self, command_name: &str) -> Option<&dyn CommandHandler> {
        self.commands.get(command_name).map(|h| h.as_ref())
    }

    pub fn get_command_description(&self, command_name: &str) -> Option<String> {
        self.commands
            .get(command_name)
            .map(|h| h.description().to_string())
    }
}

// 命令响应结构体
#[derive(Debug, serde::Serialize)]
pub struct CommandResponse {
    pub success: bool,
    pub message: String,
    pub data: Option<Value>,
}

// 处理命令的主函数
pub async fn handle_command(
    cmd: web::Json<Value>,
    data: web::Data<crate::AppState>,
    req: actix_web::HttpRequest,
) -> impl Responder {
    let command = cmd.get("command").and_then(|v| v.as_str()).unwrap_or("");
    let session_id = cmd.get("session_id").and_then(|v| v.as_str()).unwrap_or("");

    // 从 Authorization header 中获取 token
    let token = req
        .headers()
        .get("Authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.strip_prefix("Bearer "))
        .unwrap_or("");

    info!("收到命令请求: {} (session_id: {})", command, session_id);

    let args: Vec<&str> = command.split_whitespace().collect();
    if args.is_empty() {
        warn!("空命令");
        return HttpResponse::BadRequest().json(CommandResponse {
            success: false,
            message: "命令不能为空".to_string(),
            data: None,
        });
    }

    // 如果命令以 "description" 开头，返回命令描述
    if args[0] == "description" && args.len() > 1 {
        if let Some(description) = data.command_registry.get_command_description(args[1]) {
            return HttpResponse::Ok().json(CommandResponse {
                success: true,
                message: description,
                data: None,
            });
        } else {
            return HttpResponse::BadRequest().json(CommandResponse {
                success: false,
                message: format!("未知命令: {}", args[1]),
                data: None,
            });
        }
    }

    // 获取命令处理器
    if let Some(handler) = data.command_registry.get_handler(args[0]) {
        debug!("执行命令: {}", args[0]);
        // 对于需要认证的命令，使用 token；对于不需要认证的命令，使用 session_id
        let auth_token = if requires_auth(args[0]) {
            token
        } else {
            session_id
        };
        let response = handler.handle(&args, &data, auth_token).await;
        if !response.status().is_success() {
            error!("命令执行失败: {} - {}", args[0], response.status());
        }
        response
    } else {
        warn!("未知命令: {}", args[0]);
        HttpResponse::BadRequest().json(CommandResponse {
            success: false,
            message: format!("未知命令: {}", args[0]),
            data: None,
        })
    }
}

// 判断命令是否需要认证
fn requires_auth(command: &str) -> bool {
    matches!(
        command,
        "profile" | "logout" | "id" | "ls" | "pwd" | "mkdir" | "rm" | "mv"
    )
}

// 在 register_commands 函数中添加 所有 命令的注册
pub fn register_commands(registry: &mut CommandRegistry) {
    registry.register(Box::new(cmd_help::HelpCommand::new()));
    registry.register(Box::new(cmd_register::RegisterCommand::new()));
    registry.register(Box::new(cmd_login::LoginCommand::new()));
    registry.register(Box::new(cmd_logout::LogoutCommand::new()));
    registry.register(Box::new(cmd_id::IdCommand::new()));
    registry.register(Box::new(cmd_profile::ProfileCommand::new()));
    registry.register(Box::new(cmd_ls::LsCommand::new()));
    registry.register(Box::new(cmd_pwd::PwdCommand::new()));
    registry.register(Box::new(cmd_mkdir::MkdirCommand::new()));
}
