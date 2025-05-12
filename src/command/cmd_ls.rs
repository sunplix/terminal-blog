use super::CommandHandler;
use crate::auth::validate_token;
use crate::vfs::model::{Role, User as VfsUser};
use actix_web::{web, HttpResponse};
use async_trait::async_trait;
use log::{debug, error, info};
use serde_json::json;

pub struct LsCommand;

impl LsCommand {
    pub fn new() -> Self {
        LsCommand
    }
}

#[async_trait]
impl CommandHandler for LsCommand {
    fn name(&self) -> &'static str {
        "ls"
    }

    fn description(&self) -> &'static str {
        "显示目录内容，用法：ls [路径]"
    }

    async fn handle(
        &self,
        args: &[&str],
        data: &web::Data<crate::AppState>,
        token: &str,
    ) -> HttpResponse {
        info!("开始处理 ls 命令");

        // 获取路径参数，默认为当前目录
        let path = if args.len() > 1 { args[1] } else { "." };
        debug!("处理 ls 命令，路径: {}", path);

        // 验证 token
        let claims = match validate_token(token) {
            Ok(claims) => claims,
            Err(_) => {
                error!("无效的 token");
                return HttpResponse::Unauthorized().json(super::CommandResponse {
                    success: false,
                    message: "请先登录".to_string(),
                    data: None,
                });
            }
        };

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
        let user = match sqlx::query!("SELECT username, role FROM users WHERE id = $1", claims.sub)
            .fetch_optional(&data.db)
            .await
        {
            Ok(Some(user)) => VfsUser {
                id: claims.sub,
                username: user.username,
                roles: vec![match user.role.as_str() {
                    "admin" => Role::Admin,
                    "user" => Role::Author,
                    _ => Role::Guest,
                }],
            },
            Ok(None) => {
                error!("用户不存在");
                return HttpResponse::Unauthorized().json(super::CommandResponse {
                    success: false,
                    message: "用户不存在".to_string(),
                    data: None,
                });
            }
            Err(e) => {
                error!("数据库查询错误: {}", e);
                return HttpResponse::InternalServerError().json(super::CommandResponse {
                    success: false,
                    message: "服务器内部错误".to_string(),
                    data: None,
                });
            }
        };

        // 设置默认目录为用户目录
        let cwd = format!("/home/{}/", user.username);

        // 获取目录内容
        match data.vfs_manager.list_dir(&user, path, &cwd).await {
            Ok(nodes) => {
                info!("成功获取目录内容: {}", path);
                HttpResponse::Ok().json(super::CommandResponse {
                    success: true,
                    message: "目录内容获取成功".to_string(),
                    data: Some(json!({
                        "path": path,
                                "contents": nodes.iter().map(|node| {
                                    json!({
                                        "name": node.name,
                                        "is_directory": node.is_dir,
                                        "owner": node.owner_id,
                                        "permissions": format!("{:o}", node.permissions),
                                        "created_at": node.created_at,
                                        "updated_at": node.updated_at
                                    })
                                }).collect::<Vec<_>>()
                    })),
                })
            }
            Err(e) => {
                error!("获取目录内容失败: {:?}", e);
                HttpResponse::BadRequest().json(super::CommandResponse {
                    success: false,
                    message: format!("获取目录内容失败: {:?}", e),
                    data: None,
                })
            }
        }
    }
}
