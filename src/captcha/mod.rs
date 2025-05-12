mod manager;
mod types;

pub use manager::CaptchaManager;
pub use types::Captcha;

use actix_web::web;

// 重新导出处理器函数
pub use manager::get_captcha;
