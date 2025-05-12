use crate::vfs::{
    model::{Role, User as VfsUser, VfsError, VfsNode, DEFAULT_DIR_PERM},
    storage::StorageBackend,
};
use async_trait::async_trait;
use log;
use sqlx::PgPool;

pub struct PostgresBackend {
    pool: PgPool,
}

impl PostgresBackend {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// 初始化数据库表
    pub async fn init(&self) -> Result<(), VfsError> {
        // 创建表（如果不存在）
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS vfs_nodes (
                id BIGSERIAL PRIMARY KEY,
                parent_id BIGINT REFERENCES vfs_nodes(id) ON DELETE CASCADE,
                name TEXT NOT NULL,
                is_dir BOOLEAN NOT NULL,
                owner_id VARCHAR NOT NULL,
                permissions SMALLINT NOT NULL,
                created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                UNIQUE(parent_id, name)
            )
            "#,
        )
        .execute(&self.pool)
        .await
        .map_err(|e| VfsError::StorageError(format!("创建表失败: {}", e)))?;

        // 检查根目录是否存在
        let root_exists: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM vfs_nodes WHERE parent_id IS NULL AND name = '/')",
        )
        .fetch_one(&self.pool)
        .await
        .map_err(|e| VfsError::StorageError(format!("检查根目录失败: {}", e)))?;

        if !root_exists {
            // 创建根目录
            sqlx::query(
                r#"
                INSERT INTO vfs_nodes (parent_id, name, is_dir, owner_id, permissions)
                VALUES (NULL, '/', true, 'system', 755)
            "#,
            )
            .execute(&self.pool)
            .await
            .map_err(|e| VfsError::StorageError(format!("创建根目录失败: {}", e)))?;
        }

        // 检查home目录是否存在
        let home_exists: bool =
            sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM vfs_nodes WHERE name = '/home')")
                .fetch_one(&self.pool)
                .await
                .map_err(|e| VfsError::StorageError(format!("检查home目录失败: {}", e)))?;

        if !home_exists {
            // 创建home目录
            sqlx::query(
                r#"
            INSERT INTO vfs_nodes (parent_id, name, is_dir, owner_id, permissions)
                SELECT id, 'home', true, 'system', 755
            FROM vfs_nodes
                WHERE parent_id IS NULL AND name = '/'
                "#,
            )
            .execute(&self.pool)
            .await
            .map_err(|e| VfsError::StorageError(format!("创建home目录失败: {}", e)))?;
        }

        Ok(())
    }
}

#[async_trait]
impl StorageBackend for PostgresBackend {
    async fn list(&self, path: &str) -> Result<Vec<VfsNode>, VfsError> {
        // 先获取父节点
        let parent = sqlx::query!(
            r#"
            SELECT * FROM vfs_nodes WHERE name = $1
            "#,
            path
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| VfsError::StorageError(format!("查询父节点失败: {}", e)))?;

        let parent_id = match parent {
            Some(p) => p.id,
            None => return Err(VfsError::NodeNotFound(format!("目录不存在: {}", path))),
        };

        // 获取所有子节点
        let nodes = sqlx::query!(
            r#"
            SELECT * FROM vfs_nodes WHERE parent_id = $1
            "#,
            parent_id
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| VfsError::StorageError(format!("查询子节点失败: {}", e)))?;

        Ok(nodes
            .into_iter()
            .map(|n| VfsNode {
                id: n.id,
                parent_id: n.parent_id,
                name: n.name,
                is_dir: n.is_dir,
                owner_id: n.owner_id,
                permissions: n.permissions,
                created_at: n.created_at,
                updated_at: n.updated_at,
            })
            .collect())
    }

    async fn create_dir(&self, path: &str, user_id: String) -> Result<VfsNode, VfsError> {
        // 获取父目录路径
        let parent_path = if path == "/" {
            return Err(VfsError::InvalidOperation("不能创建根目录".to_string()));
        } else {
            let path = path.trim_end_matches('/');
            match path.rsplit_once('/') {
                Some((parent, _)) => {
                    if parent.is_empty() {
                        "/"
                    } else {
                        parent
                    }
                }
                None => "/",
            }
        };

        // 获取父节点
        let parent = sqlx::query!(
            r#"
            SELECT * FROM vfs_nodes WHERE name = $1
            "#,
            parent_path
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| VfsError::StorageError(format!("查询父节点失败: {}", e)))?;

        let parent_id = match parent {
            Some(p) => p.id,
            None => {
                return Err(VfsError::NodeNotFound(format!(
                    "父目录不存在: {}",
                    parent_path
                )))
            }
        };

        // 检查目录是否已存在
        let exists = sqlx::query!(
            r#"
            SELECT EXISTS(SELECT 1 FROM vfs_nodes WHERE parent_id = $1 AND name = $2)
            "#,
            parent_id,
            path
        )
        .fetch_one(&self.pool)
        .await
        .map_err(|e| VfsError::StorageError(format!("检查目录存在失败: {}", e)))?;

        if exists.exists.unwrap_or(false) {
            return Err(VfsError::NodeExists(format!("目录已存在: {}", path)));
        }

        // 如果是根目录下的用户目录，允许创建
        let is_user_home = path == "/home" || path == format!("/home/{}", user_id);

        // 创建新目录
        let node = sqlx::query!(
            r#"
            INSERT INTO vfs_nodes (parent_id, name, is_dir, owner_id, permissions)
            VALUES ($1, $2, true, $3, $4)
            RETURNING *
            "#,
            parent_id,
            path,
            user_id,
            if is_user_home {
                511
            } else {
                DEFAULT_DIR_PERM as i16
            }
        )
        .fetch_one(&self.pool)
        .await
        .map_err(|e| VfsError::StorageError(format!("创建目录失败: {}", e)))?;

        Ok(VfsNode {
            id: node.id,
            parent_id: node.parent_id,
            name: node.name,
            is_dir: node.is_dir,
            owner_id: node.owner_id,
            permissions: node.permissions,
            created_at: node.created_at,
            updated_at: node.updated_at,
        })
    }

    async fn delete(&self, path: &str) -> Result<(), VfsError> {
        let result = sqlx::query!(
            r#"
            DELETE FROM vfs_nodes WHERE name = $1
            "#,
            path
        )
        .execute(&self.pool)
        .await
        .map_err(|e| VfsError::StorageError(format!("删除节点失败: {}", e)))?;

        if result.rows_affected() == 0 {
            return Err(VfsError::NodeNotFound(format!("节点不存在: {}", path)));
        }

        Ok(())
    }

    async fn rename(&self, old_path: &str, new_path: &str) -> Result<(), VfsError> {
        let result = sqlx::query!(
            r#"
            UPDATE vfs_nodes SET name = $1 WHERE name = $2
            "#,
            new_path,
            old_path
        )
        .execute(&self.pool)
        .await
        .map_err(|e| VfsError::StorageError(format!("重命名节点失败: {}", e)))?;

        if result.rows_affected() == 0 {
            return Err(VfsError::NodeNotFound(format!("节点不存在: {}", old_path)));
        }

        Ok(())
    }

    async fn get_node(&self, path: &str) -> Result<VfsNode, VfsError> {
        let node = sqlx::query!(
            r#"
            SELECT id, parent_id, name, is_dir, owner_id, permissions, created_at, updated_at
            FROM vfs_nodes
            WHERE name = $1
            "#,
            path
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| VfsError::StorageError(format!("查询节点失败: {}", e)))?;

        match node {
            Some(n) => Ok(VfsNode {
                id: n.id,
                parent_id: n.parent_id,
                name: n.name,
                is_dir: n.is_dir,
                owner_id: n.owner_id,
                permissions: n.permissions,
                created_at: n.created_at,
                updated_at: n.updated_at,
            }),
            None => Err(VfsError::NodeNotFound(format!("节点不存在: {}", path))),
        }
    }

    async fn update_node(&self, node: &VfsNode) -> Result<(), VfsError> {
        let result = sqlx::query!(
            r#"
            UPDATE vfs_nodes
            SET permissions = $1, updated_at = NOW()
            WHERE id = $2
            "#,
            node.permissions,
            node.id
        )
        .execute(&self.pool)
        .await
        .map_err(|e| VfsError::StorageError(format!("更新节点失败: {}", e)))?;

        if result.rows_affected() == 0 {
            return Err(VfsError::NodeNotFound(format!("节点不存在: {}", node.name)));
        }

        Ok(())
    }
}
