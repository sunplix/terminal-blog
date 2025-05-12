use crate::vfs::{
    model::{User, VfsError, VfsNode, VfsOp},
    path_normalizer::PathNormalizer,
    permission::PermissionManager,
    storage::StorageBackend,
};
use log::{debug, info, warn};

pub struct VfsManager<B: StorageBackend> {
    backend: B,
}

impl<B: StorageBackend> VfsManager<B> {
    pub fn new(backend: B) -> Self {
        Self { backend }
    }

    /// 列出目录内容
    pub async fn list_dir(
        &self,
        user: &User,
        raw_path: &str,
        cwd: &str,
    ) -> Result<Vec<VfsNode>, VfsError> {
        info!("列出目录内容: {}, 用户名: {}", raw_path, user.username);

        // 规范化路径
        let path = PathNormalizer::normalize(raw_path, cwd)?;
        debug!("规范化后的路径: {}", path);

        // 获取目录节点
        let node = self.backend.get_node(&path).await?;

        // 检查权限
        PermissionManager::check(user, &node, &VfsOp::ReadDir)?;

        // 获取目录内容
        let contents = self.backend.list(&path).await?;
        info!("成功获取目录 {} 的内容", path);

        Ok(contents)
    }

    /// 创建目录
    pub async fn create_dir(
        &self,
        user: &User,
        raw_path: &str,
        cwd: &str,
    ) -> Result<VfsNode, VfsError> {
        info!("创建目录: {}, 用户名: {}", raw_path, user.username);

        // 规范化路径
        let path = PathNormalizer::normalize(raw_path, cwd)?;
        debug!("规范化后的路径: {}", path);

        // 检查父目录是否存在
        let parent_path = PathNormalizer::parent(&path)
            .ok_or_else(|| VfsError::PathError("无法获取父目录".to_string()))?;

        let parent = self.backend.get_node(&parent_path).await?;

        // 检查权限
        debug!("检查用户权限 - 用户ID: {}, 角色: {:?}", user.id, user.roles);
        if !PermissionManager::can_write(user, &parent_path) {
            warn!(
                "权限检查失败 - 用户: {}, 路径: {}",
                user.username, parent_path
            );
            return Err(VfsError::PermissionError(
                "作者用户只能在自己的目录下执行写操作".to_string(),
            ));
        }

        // 创建目录
        debug!("开始创建目录 - 用户: {}, 路径: {}", user.username, path);
        let node = self.backend.create_dir(&path, user.id.clone()).await?;
        info!("成功创建目录: {}", path);

        Ok(node)
    }

    /// 删除节点
    pub async fn delete(&self, user: &User, raw_path: &str, cwd: &str) -> Result<(), VfsError> {
        info!("删除节点: {}, 用户名: {}", raw_path, user.username);

        // 规范化路径
        let path = PathNormalizer::normalize(raw_path, cwd)?;
        debug!("规范化后的路径: {}", path);

        // 获取节点
        let node = self.backend.get_node(&path).await?;

        // 检查权限
        PermissionManager::check(user, &node, &VfsOp::Delete)?;

        // 删除节点
        self.backend.delete(&path).await?;
        info!("成功删除节点: {}", path);

        Ok(())
    }

    /// 重命名节点
    pub async fn rename(
        &self,
        user: &User,
        old_path: &str,
        new_path: &str,
        cwd: &str,
    ) -> Result<(), VfsError> {
        info!(
            "重命名节点: {} -> {}, 用户名: {}",
            old_path, new_path, user.username
        );

        // 规范化路径
        let old_path = PathNormalizer::normalize(old_path, cwd)?;
        let new_path = PathNormalizer::normalize(new_path, cwd)?;
        debug!("规范化后的路径: {} -> {}", old_path, new_path);

        // 获取节点
        let node = self.backend.get_node(&old_path).await?;

        // 检查权限
        PermissionManager::check(user, &node, &VfsOp::Rename)?;

        // 重命名节点
        self.backend.rename(&old_path, &new_path).await?;
        info!("成功重命名节点: {} -> {}", old_path, new_path);

        Ok(())
    }

    /// 获取当前工作目录
    pub fn pwd(&self, cwd: &str) -> String {
        cwd.to_string()
    }

    /// 更新节点所有者
    pub async fn update_node_owner(&self, path: &str, new_owner_id: &str) -> Result<(), VfsError> {
        debug!(
            "更新节点所有者 - 路径: {}, 新所有者: {}",
            path, new_owner_id
        );

        // 获取节点
        let mut node = self.backend.get_node(path).await?;

        // 更新所有者
        node.owner_id = new_owner_id.to_string();
        self.backend.update_node(&node).await?;

        debug!("成功更新节点所有者");
        Ok(())
    }
}
