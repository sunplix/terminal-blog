use crate::vfs::model::{Role, User, VfsError, VfsNode, VfsOp, PERM_EXEC, PERM_READ, PERM_WRITE};
use log::{debug, warn};

pub struct PermissionManager;

impl PermissionManager {
    /// 检查用户对节点的操作权限
    pub fn check(user: &User, node: &VfsNode, op: &VfsOp) -> Result<(), VfsError> {
        debug!(
            "权限检查 - 用户: {}, 节点: {}, 操作: {:?}",
            user.username, node.name, op
        );

        // Admin 角色拥有所有权限
        if user.roles.contains(&Role::Admin) {
            debug!("用户是管理员，允许所有操作");
            return Ok(());
        }

        // 检查操作类型对应的权限位
        let required_perm = match op {
            VfsOp::ReadDir | VfsOp::ReadFile => PERM_READ,
            VfsOp::WriteFile | VfsOp::CreateDir | VfsOp::Delete | VfsOp::Rename => PERM_WRITE,
            VfsOp::Execute => PERM_EXEC,
        };

        // 检查用户是否是节点所有者
        let is_owner = user.id == node.owner_id;
        debug!("是否是节点所有者: {}", is_owner);

        // 获取权限位
        let perms = node.permissions;
        let owner_perm = (perms >> 6) & 0o7;
        let group_perm = (perms >> 3) & 0o7;
        let other_perm = perms & 0o7;

        // 检查权限
        let has_permission = if is_owner {
            (owner_perm & required_perm) == required_perm
        } else {
            // 这里简化处理，实际应该检查用户组
            (other_perm & required_perm) == required_perm
        };

        if !has_permission {
            return Err(VfsError::PermissionError(format!(
                "用户 {} 没有执行 {:?} 操作的权限",
                user.username, op
            )));
        }

        // 特殊路径检查
        if !Self::check_special_paths(user, node, op)? {
            return Err(VfsError::PermissionError(format!(
                "用户 {} 不能在此路径执行 {:?} 操作",
                user.username, op
            )));
        }

        Ok(())
    }

    /// 检查特殊路径的权限
    fn check_special_paths(user: &User, node: &VfsNode, op: &VfsOp) -> Result<bool, VfsError> {
        // Guest 用户只能访问 /home/guest/
        if user.roles.contains(&Role::Guest) {
            if !node.name.starts_with("/home/guest/") {
                return Err(VfsError::PermissionError(
                    "访客用户只能访问 /home/guest/ 目录".to_string(),
                ));
            }
            // Guest 用户只有读权限
            if op != &VfsOp::ReadDir && op != &VfsOp::ReadFile {
                return Err(VfsError::PermissionError("访客用户只有读权限".to_string()));
            }
        }

        // Author 用户只能在自己的目录下写操作
        if user.roles.contains(&Role::Author) {
            let user_home = format!("/home/{}/", user.username);
            if !node.name.starts_with(&user_home) {
                match op {
                    VfsOp::WriteFile | VfsOp::CreateDir | VfsOp::Delete | VfsOp::Rename => {
                        return Err(VfsError::PermissionError(
                            "作者用户只能在自己的目录下执行写操作".to_string(),
                        ));
                    }
                    _ => {}
                }
            }
        }

        Ok(true)
    }

    ///TODO: 需要使用数据库中的权限来检查。
    pub fn can_enter(user: &User, path: &str) -> Result<(), VfsError> {
        debug!("检查目录进入权限 - 用户: {}, 路径: {}", user.username, path);

        // 管理员有所有权限
        if user.roles.contains(&Role::Admin) {
            debug!("用户是管理员，允许所有操作");
            return Ok(());
        }

        // Guest 用户只能进入 /home/guest/
        if user.roles.contains(&Role::Guest) {
            if !path.starts_with("/home/guest/") {
                return Err(VfsError::PermissionError(
                    "访客用户只能进入 /home/guest/ 目录".to_string(),
                ));
            }
        }

        // Author 用户只能进入自己的目录
        if user.roles.contains(&Role::Author) {
            let user_home = format!("/home/{}", user.username);
            if !path.starts_with(&user_home) {
                return Err(VfsError::PermissionError(
                    "作者用户只能进入自己的目录".to_string(),
                ));
            }
        }

        Ok(())
    }

    pub fn can_write(user: &User, path: &str) -> bool {
        debug!("检查写权限 - 用户: {}, 路径: {}", user.username, path);

        // 管理员有所有权限
        if user.roles.contains(&Role::Admin) {
            debug!("用户是管理员，允许所有操作");
            return true;
        }

        // 普通作者只能在自己家目录及其子目录下写
        // 正确的家目录形式："/home/username" 或者以 "/home/username/" 开头
        let user_home = format!("/home/{}", user.username);
        let user_home_prefix = format!("{}/", user_home);

        let allow = path == user_home || path.starts_with(&user_home_prefix);
        debug!("是否允许写入家目录及子目录: {}", allow);
        allow
    }
}
