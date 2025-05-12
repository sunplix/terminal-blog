use super::CommandHandler;
use crate::auth::validate_token;
use crate::vfs::model::{Role, User as VfsUser, VfsError};
use actix_web::{web, HttpResponse};
use async_trait::async_trait;
use log::{debug, error, info};
use serde_json::json;

pub struct MkdirCommand;

impl MkdirCommand {
    pub fn new() -> Self {
        MkdirCommand
    }
}

#[async_trait]
impl CommandHandler for MkdirCommand {
    fn name(&self) -> &'static str {
        "mkdir"
    }

    fn description(&self) -> &'static str {
        "创建目录，用法：mkdir [-p] <目录名>"
    }

    async fn handle(
        &self,
        args: &[&str],
        data: &web::Data<crate::AppState>,
        token: &str,
    ) -> HttpResponse {
        info!("开始处理 mkdir 命令");

        // 参数检查
        if args.len() < 2 {
            error!("mkdir 命令缺少参数");
            return HttpResponse::BadRequest().json(super::CommandResponse {
                success: false,
                message: "用法：mkdir [-p] <目录名>".to_string(),
                data: None,
            });
        }

        // 解析 -p 选项
        let mut recursive = false;
        let mut dir_name = args[1];
        if args[1] == "-p" {
            if args.len() < 3 {
                error!("mkdir -p 命令缺少目录名");
                return HttpResponse::BadRequest().json(super::CommandResponse {
                    success: false,
                    message: "用法：mkdir -p <目录名>".to_string(),
                    data: None,
                });
            }
            recursive = true;
            dir_name = args[2];
        }

        debug!("创建目录: {}, 递归: {}", dir_name, recursive);

        // 验证 JWT
        let claims = match validate_token(token) {
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
        if data.auth_manager.is_token_blacklisted(token) {
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

        // 家目录，用于相对路径
        let cwd = format!("/home/{}/", user.username);

        if recursive {
            // 拆分路径组件
            let parts: Vec<&str> = dir_name
                .trim_end_matches('/')
                .split('/')
                .filter(|s| !s.is_empty())
                .collect();

            // full：当前要操作的完整路径；parent_cwd：传给 normalize & create 的上下文
            let mut full = String::new();
            let mut parent_cwd = if dir_name.starts_with('/') {
                // 绝对路径，从根 "/" 开始
                full.push('/');
                "/".to_string()
            } else {
                // 相对路径，从用户家目录开始
                full = cwd.trim_end_matches('/').to_string();
                cwd.clone()
            };

            for part in parts {
                // 拼接当前一级目录
                if full != "/" {
                    full.push('/');
                }
                full.push_str(part);

                // 先检查目录是否存在（读权限+存在性）
                match data.vfs_manager.list_dir(&user, &full, &parent_cwd).await {
                    Ok(_) => {
                        debug!("目录已存在，跳过: {}", full);
                    }
                    Err(VfsError::NodeNotFound(_)) => {
                        // 不存在才去创建
                        match data.vfs_manager.create_dir(&user, &full, &parent_cwd).await {
                            Ok(_) => debug!("成功创建目录: {}", full),
                            Err(e) => {
                                error!("创建目录失败: {:?}", e);
                                return HttpResponse::BadRequest().json(super::CommandResponse {
                                    success: false,
                                    message: format!("创建目录失败: {:?}", e),
                                    data: None,
                                });
                            }
                        }
                    }
                    Err(e) => {
                        // 其他错误（权限/路径错误等）
                        error!("检查目录状态失败: {:?}", e);
                        return HttpResponse::InternalServerError().json(super::CommandResponse {
                            success: false,
                            message: "服务器内部错误".to_string(),
                            data: None,
                        });
                    }
                }

                // 更新上下文 cwd
                parent_cwd = full.clone();
            }

            HttpResponse::Ok().json(super::CommandResponse {
                success: true,
                message: format!("目录 {} 创建成功", dir_name),
                data: None,
            })
        } else {
            // 非递归创建
            match data.vfs_manager.create_dir(&user, dir_name, &cwd).await {
                Ok(node) => {
                    info!("成功创建目录: {}", dir_name);
                    HttpResponse::Ok().json(super::CommandResponse {
                        success: true,
                        message: format!("目录 {} 创建成功", dir_name),
                        data: Some(json!({
                            "name": node.name,
                            "is_directory": node.is_dir,
                            "owner": node.owner_id,
                            "permissions": format!("{:o}", node.permissions),
                            "created_at": node.created_at,
                            "updated_at": node.updated_at
                        })),
                    })
                }
                Err(e) => {
                    error!("创建目录失败: {:?}", e);
                    HttpResponse::BadRequest().json(super::CommandResponse {
                        success: false,
                        message: format!("创建目录失败: {:?}", e),
                        data: None,
                    })
                }
            }
        }
    }
}
