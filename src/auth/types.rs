use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Claims {
    pub sub: String, // user id
    pub exp: usize,  // expiration time
    pub id: String,
    pub username: String,
    pub role: String,
}

pub struct LoginAttempt {
    pub count: i32,
    pub last_attempt: DateTime<Utc>,
}
