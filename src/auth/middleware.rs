use actix_web::{
    dev::{forward_ready, Service, ServiceRequest, ServiceResponse, Transform},
    error::ErrorUnauthorized,
    Error, HttpMessage,
};
use futures::future::{ready, LocalBoxFuture, Ready};
use log::{debug, warn};
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use crate::auth::{validate_token, AuthManager, Claims};

#[derive(Clone)]
pub struct AuthMiddleware {
    auth_manager: Arc<AuthManager>,
}

impl AuthMiddleware {
    pub fn new() -> Self {
        Self {
            auth_manager: Arc::new(AuthManager::new()),
        }
    }
}

impl<S, B> Transform<S, ServiceRequest> for AuthMiddleware
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Transform = AuthMiddlewareService<S>;
    type InitError = ();
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ready(Ok(AuthMiddlewareService {
            service: Arc::new(service),
            auth_manager: self.auth_manager.clone(),
        }))
    }
}

pub struct AuthMiddlewareService<S> {
    service: Arc<S>,
    auth_manager: Arc<AuthManager>,
}

impl<S, B> Service<ServiceRequest> for AuthMiddlewareService<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

    forward_ready!(service);

    fn call(&self, req: ServiceRequest) -> Self::Future {
        let auth_manager = self.auth_manager.clone();
        let service = self.service.clone();

        Box::pin(async move {
            // 从请求头中获取 token
            let token = req
                .headers()
                .get("Authorization")
                .and_then(|v| v.to_str().ok())
                .and_then(|s| s.strip_prefix("Bearer "))
                .unwrap_or("");

            if token.is_empty() {
                warn!("未提供认证 token");
                return Err(ErrorUnauthorized("未提供认证 token"));
            }

            // 验证 token
            match validate_token(token) {
                Ok(claims) => {
                    // 检查 token 是否在黑名单中
                    if auth_manager.is_token_blacklisted(token) {
                        warn!("Token 已失效");
                        return Err(ErrorUnauthorized("Token 已失效"));
                    }

                    debug!("用户 {} 认证成功", claims.username);

                    // 将用户信息添加到请求扩展中
                    let mut req = req;
                    req.extensions_mut().insert(claims);

                    service.call(req).await
                }
                Err(e) => {
                    warn!("Token 验证失败: {}", e);
                    Err(ErrorUnauthorized("Token 验证失败"))
                }
            }
        })
    }
}
