use actix_web::web;
use chrono::{DateTime, Utc};
use log::{debug, error, info, warn};
use rand::{thread_rng, Rng};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use uuid::Uuid;

use super::types::Captcha;

// 验证码管理器
#[derive(Clone)]
pub struct CaptchaManager {
    captchas: Arc<Mutex<HashMap<String, Captcha>>>,
}

impl CaptchaManager {
    pub fn new() -> Self {
        info!("初始化验证码管理器");
        Self {
            captchas: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    // 生成验证码
    pub fn generate_captcha() -> String {
        debug!("生成新的验证码");
        let mut rng = thread_rng();
        // 生成6位验证码，包含数字和大写字母
        let chars: String = (0..6)
            .map(|_| {
                if rng.gen_bool(0.5) {
                    // 生成数字
                    rng.gen_range(0..10).to_string()
                } else {
                    // 生成大写字母
                    (rng.gen_range(b'A'..=b'Z') as char).to_string()
                }
            })
            .collect();
        info!("生成验证码: {}", chars);
        chars
    }

    // 验证验证码
    pub fn verify_captcha(&self, session_id: &str, code: &str) -> bool {
        debug!("验证验证码 - 会话ID: {}, 验证码: {}", session_id, code);
        let mut captchas = self.captchas.lock().unwrap();
        if let Some(captcha) = captchas.get_mut(session_id) {
            // 检查验证码是否过期（5分钟）
            let now = Utc::now();
            if (now - captcha.created_at).num_seconds() > 300 {
                warn!("验证码已过期 - 会话ID: {}", session_id);
                captchas.remove(session_id);
                return false;
            }

            // 检查尝试次数
            if captcha.attempts >= 3 {
                warn!("验证码尝试次数过多 - 会话ID: {}", session_id);
                captchas.remove(session_id);
                return false;
            }

            // 增加尝试次数
            captcha.attempts += 1;
            debug!(
                "验证码尝试次数: {} - 会话ID: {}",
                captcha.attempts, session_id
            );

            // 验证码正确
            if captcha.code == code {
                info!("验证码验证成功 - 会话ID: {}", session_id);
                return true;
            } else {
                warn!("验证码错误 - 会话ID: {}", session_id);
            }
        } else {
            warn!("未找到验证码记录 - 会话ID: {}", session_id);
        }
        false
    }

    // 清理过期的验证码
    pub fn cleanup_expired_captchas(&self) {
        debug!("清理过期的验证码");
        let mut captchas = self.captchas.lock().unwrap();
        let before_count = captchas.len();
        let now = Utc::now();
        captchas.retain(|_, captcha| (now - captcha.created_at).num_seconds() <= 300);
        let after_count = captchas.len();
        info!("清理了 {} 个过期的验证码", before_count - after_count);
    }
}

// 获取验证码的处理器
pub async fn get_captcha(data: web::Data<crate::AppState>) -> impl actix_web::Responder {
    debug!("处理获取验证码请求");
    let session_id = Uuid::new_v4().to_string();
    let code = CaptchaManager::generate_captcha();

    let mut captchas = data.captcha_manager.captchas.lock().unwrap();
    captchas.insert(
        session_id.clone(),
        Captcha {
            code: code.clone(),
            created_at: Utc::now(),
            attempts: 0,
        },
    );

    info!("生成新的验证码 - 会话ID: {}", session_id);
    actix_web::HttpResponse::Ok().json(serde_json::json!({
        "success": true,
        "data": {
            "session_id": session_id,
            "captcha": code
        }
    }))
}
