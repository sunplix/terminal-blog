use crate::vfs::model::{VfsError, VfsNode};
use async_trait::async_trait;

#[async_trait]
pub trait StorageBackend: Send + Sync {
    /// 列出目录内容
    async fn list(&self, path: &str) -> Result<Vec<VfsNode>, VfsError>;

    /// 创建目录
    async fn create_dir(&self, path: &str, user_id: String) -> Result<VfsNode, VfsError>;

    /// 删除节点
    async fn delete(&self, path: &str) -> Result<(), VfsError>;

    /// 重命名节点
    async fn rename(&self, old_path: &str, new_path: &str) -> Result<(), VfsError>;

    /// 获取节点信息
    async fn get_node(&self, path: &str) -> Result<VfsNode, VfsError>;

    /// 更新节点信息
    async fn update_node(&self, node: &VfsNode) -> Result<(), VfsError>;
}

pub mod backend;
