use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fmt;
use thiserror::Error;

pub type NodeId = i64;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VfsNode {
    pub id: NodeId,
    pub parent_id: Option<NodeId>,
    pub name: String,
    pub is_dir: bool,
    pub owner_id: String,
    pub permissions: i16, // Unix 模式 bits
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: String,
    pub username: String,
    pub roles: Vec<Role>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Role {
    Admin,
    Author,
    Guest,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum VfsOp {
    ReadDir,
    ReadFile,
    WriteFile,
    CreateDir,
    Delete,
    Rename,
}

#[derive(Debug, Error)]
pub enum VfsError {
    #[error("路径错误: {0}")]
    PathError(String),

    #[error("权限错误: {0}")]
    PermissionError(String),

    #[error("存储错误: {0}")]
    StorageError(String),

    #[error("节点不存在: {0}")]
    NodeNotFound(String),

    #[error("节点已存在: {0}")]
    NodeExists(String),

    #[error("无效操作: {0}")]
    InvalidOperation(String),
}

// 权限常量
pub const PERM_READ: i16 = 0o4;
pub const PERM_WRITE: i16 = 0o2;
pub const PERM_EXEC: i16 = 0o1;

// 默认权限
pub const DEFAULT_DIR_PERM: i16 = 0o755; // rwxr-xr-x
pub const DEFAULT_FILE_PERM: i16 = 0o644; // rw-r--r--
