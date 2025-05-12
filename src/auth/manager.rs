use bcrypt::{hash, verify, DEFAULT_COST};
use chrono::{DateTime, Utc};
use log::{debug, error, info, warn};
use std::collections::HashMap;
use std::sync::Mutex;

use super::types::LoginAttempt;

pub struct AuthManager {
    login_attempts: Mutex<HashMap<String, LoginAttempt>>,
    token_blacklist: Mutex<HashMap<String, DateTime<Utc>>>,
}

impl AuthManager {
    pub fn new() -> Self {
        info!("初始化认证管理器");
        Self {
            login_attempts: Mutex::new(HashMap::new()),
            token_blacklist: Mutex::new(HashMap::new()),
        }
    }

    pub fn check_login_attempts(&self, username: &str) -> Result<(), String> {
        debug!("检查用户 {} 的登录尝试次数", username);
        let mut attempts = self.login_attempts.lock().unwrap();
        let attempt = attempts
            .entry(username.to_string())
            .or_insert(LoginAttempt {
                count: 0,
                last_attempt: Utc::now(),
            });

        // 如果失败次数过多，检查是否需要等待
        if attempt.count >= 5 {
            let wait_time = chrono::Duration::minutes(15);
            if Utc::now() - attempt.last_attempt < wait_time {
                warn!("用户 {} 登录尝试次数过多", username);
                return Err(format!(
                    "登录尝试次数过多，请等待 {} 分钟后再试",
                    (wait_time - (Utc::now() - attempt.last_attempt)).num_minutes()
                ));
            }
            // 重置计数器
            attempt.count = 0;
            info!("用户 {} 的登录尝试次数已重置", username);
        }
        Ok(())
    }

    pub fn record_failed_attempt(&self, username: &str) {
        debug!("记录用户 {} 的登录失败", username);
        let mut attempts = self.login_attempts.lock().unwrap();
        let attempt = attempts
            .entry(username.to_string())
            .or_insert(LoginAttempt {
                count: 0,
                last_attempt: Utc::now(),
            });
        attempt.count += 1;
        attempt.last_attempt = Utc::now();
        warn!(
            "用户 {} 登录失败，当前失败次数: {}",
            username, attempt.count
        );
    }

    pub fn reset_login_attempts(&self, username: &str) {
        debug!("重置用户 {} 的登录尝试次数", username);
        let mut attempts = self.login_attempts.lock().unwrap();
        if let Some(attempt) = attempts.get_mut(username) {
            attempt.count = 0;
            info!("用户 {} 的登录尝试次数已重置", username);
        }
    }

    pub fn blacklist_token(&self, token: &str) {
        debug!("将 token 加入黑名单");
        let mut blacklist = self.token_blacklist.lock().unwrap();
        blacklist.insert(token.to_string(), Utc::now());
        info!("Token 已加入黑名单");
    }

    pub fn is_token_blacklisted(&self, token: &str) -> bool {
        debug!("检查 token 是否在黑名单中");
        let blacklist = self.token_blacklist.lock().unwrap();
        let is_blacklisted = blacklist.contains_key(token);
        if is_blacklisted {
            warn!("Token 在黑名单中");
        }
        is_blacklisted
    }

    pub fn cleanup_expired_tokens(&self) {
        debug!("清理过期的 token");
        let mut blacklist = self.token_blacklist.lock().unwrap();
        let before_count = blacklist.len();
        blacklist.retain(|_, timestamp| Utc::now() - *timestamp <= chrono::Duration::hours(24));
        let after_count = blacklist.len();
        info!("清理了 {} 个过期的 token", before_count - after_count);
    }

    pub fn hash_password(password: &str) -> Result<String, String> {
        debug!("加密密码");
        match hash(password.as_bytes(), DEFAULT_COST) {
            Ok(hash) => {
                info!("密码加密成功");
                Ok(hash)
            }
            Err(e) => {
                error!("密码加密失败: {}", e);
                Err(format!("密码加密失败: {}", e))
            }
        }
    }

    pub fn verify_password(password: &str, hash: &str) -> Result<bool, String> {
        debug!("验证密码");
        match verify(password, hash) {
            Ok(result) => {
                if result {
                    info!("密码验证成功");
                } else {
                    warn!("密码验证失败");
                }
                Ok(result)
            }
            Err(e) => {
                error!("密码验证过程出错: {}", e);
                Err(format!("密码验证失败: {}", e))
            }
        }
    }
}
