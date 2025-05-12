use crate::auth::types::Claims;
use chrono::Utc;
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use log::{debug, error, info};
use std::env;

pub fn validate_token(token: &str) -> Result<Claims, jsonwebtoken::errors::Error> {
    debug!("验证 JWT token");
    decode::<Claims>(
        token,
        &DecodingKey::from_secret(env::var("JWT_SECRET").unwrap().as_bytes()),
        &Validation::default(),
    )
    .map(|data| data.claims)
}

pub fn generate_token(
    user_id: &str,
    username: &str,
    role: &str,
) -> Result<String, jsonwebtoken::errors::Error> {
    debug!("为用户 {} 生成 JWT token", user_id);
    let exp = (Utc::now() + chrono::Duration::hours(24)).timestamp() as usize;
    let claims = Claims {
        sub: user_id.to_string(),
        exp,
        id: user_id.to_string(),
        username: username.to_string(),
        role: role.to_string(),
    };

    match encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(env::var("JWT_SECRET").unwrap().as_bytes()),
    ) {
        Ok(token) => {
            info!("成功为用户 {} 生成 token", user_id);
            Ok(token)
        }
        Err(e) => {
            error!("为用户 {} 生成 token 失败: {}", user_id, e);
            Err(e)
        }
    }
}
