use super::CommandHandler;
use crate::auth::validate_token;
use crate::vfs::model::{Role, User as VfsUser, VfsError, VfsOp};
use crate::vfs::permission::PermissionManager;
use actix_web::{web, HttpResponse};
use async_trait::async_trait;
use log::{debug, error, info};
use serde_json::json;
use std::path::{Path, PathBuf};

pub struct CdCommand;

impl CdCommand {
    pub fn new() -> Self {
        CdCommand
    }

    fn normalize_path(current_path: &str, target_path: &str) -> String {
        let mut result = PathBuf::from(current_path);

        // 如果是绝对路径，直接使用target_path
        if target_path.starts_with('/') {
            result = PathBuf::from(target_path);
        } else {
            // 否则拼接当前路径和目标路径
            result.push(target_path);
        }

        // 规范化路径
        let mut components = result.components().collect::<Vec<_>>();
        let mut normalized_path = PathBuf::new();

        for component in components {
            match component {
                // 跳过当前目录 '.'
                std::path::Component::CurDir => {}
                // 如果是父级目录 '..'，则移除上一个有效的路径部分
                std::path::Component::ParentDir => {
                    normalized_path.pop();
                }
                // 其他情况，将路径部分添加到规范路径中
                _ => normalized_path.push(component),
            }
        }

        normalized_path.to_string_lossy().to_string()
    }
}

#[async_trait]
impl CommandHandler for CdCommand {
    fn name(&self) -> &'static str {
        "cd"
    }

    fn description(&self) -> &'static str {
        "切换当前工作目录"
    }

    async fn handle(
        &self,
        args: &[&str],
        data: &web::Data<crate::AppState>,
        session_id: &str,
        _cwd: &str,
    ) -> HttpResponse {
        // 检查参数
        if args.len() != 2 {
            return HttpResponse::BadRequest().json(super::CommandResponse {
                success: false,
                message: "用法: cd <目录路径>".to_string(),
                data: None,
            });
        }

        let target_path = Self::normalize_path(_cwd, args[1]);
        let normalized_path = target_path.replace("\\", "/");

        debug!("切换目录: {} -> {}", _cwd, normalized_path);

        // 验证 JWT
        let claims = match validate_token(session_id) {
            Ok(c) => c,
            Err(_) => {
                error!("无效的 token");
                return HttpResponse::Unauthorized().json(super::CommandResponse {
                    success: false,
                    message: "请先登录".to_string(),
                    data: None,
                });
            }
        };
        if data.auth_manager.is_token_blacklisted(session_id) {
            debug!("Token 已失效");
            return HttpResponse::Unauthorized().json(super::CommandResponse {
                success: false,
                message: "Token 已失效".to_string(),
                data: None,
            });
        }

        // 查询用户身份
        let user = match sqlx::query!("SELECT username, role FROM users WHERE id = $1", claims.sub)
            .fetch_optional(&data.db)
            .await
        {
            Ok(Some(rec)) => VfsUser {
                id: claims.sub.clone(),
                username: rec.username,
                roles: vec![match rec.role.as_str() {
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

        // 检查用户是否有权限进入目标目录
        let result = PermissionManager::can_enter(&user, &normalized_path);
        match result {
            Ok(_) => {
                return HttpResponse::Ok().json(super::CommandResponse {
                    success: true,
                    message: "目录切换成功".to_string(),
                    data: Some(serde_json::json!({
                        "path": normalized_path
                    })),
                })
            }
            Err(e) => {
                return HttpResponse::Forbidden().json(super::CommandResponse {
                    success: false,
                    message: e.to_string(),
                    data: None,
                });
            }
        }
    }
}
