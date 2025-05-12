use crate::vfs::model::VfsError;
use log::{error, info};
use sqlx::PgPool;

pub struct DbInitializer {
    pool: PgPool,
}

impl DbInitializer {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// 初始化用户表
    pub async fn init_user_tables(&self) -> Result<(), VfsError> {
        // 创建用户表
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS users (
                id VARCHAR PRIMARY KEY,
                username VARCHAR UNIQUE NOT NULL,
                password_hash VARCHAR NOT NULL,
                email VARCHAR,
                gender VARCHAR CHECK (gender IN ('male', 'female', 'other')),
                birthday DATE,
                role VARCHAR NOT NULL DEFAULT 'user' CHECK (role IN ('admin', 'user')),
                created_at TIMESTAMP WITH TIME ZONE NOT NULL
            )
            "#,
        )
        .execute(&self.pool)
        .await
        .map_err(|e| VfsError::StorageError(format!("创建用户表失败: {}", e)))?;

        // 检查是否需要添加列
        let check_columns = sqlx::query!(
            r#"
            SELECT column_name 
            FROM information_schema.columns 
            WHERE table_name = 'users'
            "#
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| VfsError::StorageError(format!("检查用户表列失败: {}", e)))?;

        let existing_columns: Vec<String> = check_columns
            .iter()
            .filter_map(|row| row.column_name.clone())
            .collect();

        // 添加缺失的列
        if !existing_columns.contains(&"email".to_string()) {
            sqlx::query("ALTER TABLE users ADD COLUMN email VARCHAR")
                .execute(&self.pool)
                .await
                .map_err(|e| VfsError::StorageError(format!("添加email列失败: {}", e)))?;
        }

        if !existing_columns.contains(&"gender".to_string()) {
            sqlx::query("ALTER TABLE users ADD COLUMN gender VARCHAR CHECK (gender IN ('male', 'female', 'other'))")
                .execute(&self.pool)
                .await
                .map_err(|e| VfsError::StorageError(format!("添加gender列失败: {}", e)))?;
        }

        if !existing_columns.contains(&"birthday".to_string()) {
            sqlx::query("ALTER TABLE users ADD COLUMN birthday DATE")
                .execute(&self.pool)
                .await
                .map_err(|e| VfsError::StorageError(format!("添加birthday列失败: {}", e)))?;
        }

        if !existing_columns.contains(&"role".to_string()) {
            sqlx::query("ALTER TABLE users ADD COLUMN role VARCHAR NOT NULL DEFAULT 'user' CHECK (role IN ('admin', 'user'))")
                .execute(&self.pool)
                .await
                .map_err(|e| VfsError::StorageError(format!("添加role列失败: {}", e)))?;
        }

        Ok(())
    }

    /// 初始化VFS数据库表
    pub async fn init_vfs_tables(&self) -> Result<(), VfsError> {
        // 创建VFS节点表
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

        // 创建索引
        sqlx::query(
            r#"
            CREATE INDEX IF NOT EXISTS idx_vfs_parent ON vfs_nodes(parent_id)
            "#,
        )
        .execute(&self.pool)
        .await
        .map_err(|e| VfsError::StorageError(format!("创建索引失败: {}", e)))?;

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

    /// 初始化权限相关表
    pub async fn init_permission_tables(&self) -> Result<(), VfsError> {
        // 创建用户组表
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS user_groups (
                id SERIAL PRIMARY KEY,
                name VARCHAR NOT NULL UNIQUE,
                created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
            )
            "#,
        )
        .execute(&self.pool)
        .await
        .map_err(|e| VfsError::StorageError(format!("创建用户组表失败: {}", e)))?;

        // 创建用户-组关系表
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS user_group_members (
                user_id VARCHAR NOT NULL,
                group_id INTEGER NOT NULL REFERENCES user_groups(id),
                created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                PRIMARY KEY (user_id, group_id)
            )
            "#,
        )
        .execute(&self.pool)
        .await
        .map_err(|e| VfsError::StorageError(format!("创建用户-组关系表失败: {}", e)))?;

        // 创建权限审计日志表
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS permission_audit_logs (
                id BIGSERIAL PRIMARY KEY,
                user_id VARCHAR NOT NULL,
                node_id BIGINT NOT NULL REFERENCES vfs_nodes(id),
                operation VARCHAR NOT NULL,
                success BOOLEAN NOT NULL,
                created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
            )
            "#,
        )
        .execute(&self.pool)
        .await
        .map_err(|e| VfsError::StorageError(format!("创建权限审计日志表失败: {}", e)))?;

        Ok(())
    }
}

/// 初始化所有数据库表
pub async fn initialize_db(pool: PgPool) -> Result<(), VfsError> {
    info!("开始初始化数据库...");
    let initializer = DbInitializer::new(pool);

    initializer.init_user_tables().await.map_err(|e| {
        error!("用户表初始化失败: {:?}", e);
        e
    })?;

    initializer.init_vfs_tables().await.map_err(|e| {
        error!("VFS表初始化失败: {:?}", e);
        e
    })?;

    initializer.init_permission_tables().await.map_err(|e| {
        error!("权限表初始化失败: {:?}", e);
        e
    })?;

    info!("数据库初始化完成");
    Ok(())
}
