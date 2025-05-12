use chrono::{DateTime, Utc};

// 验证码结构体
#[derive(Debug, Clone)]
pub struct Captcha {
    pub code: String,
    pub created_at: DateTime<Utc>,
    pub attempts: i32,
}
