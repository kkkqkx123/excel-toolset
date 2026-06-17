#![allow(dead_code, unused)]

pub mod validation;

use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};

use axum::extract::Request;
use axum::response::Response;
use tower::Layer;

pub struct ValidationLayer;

impl<S> Layer<S> for ValidationLayer {
    type Service = ValidationMiddleware<S>;

    fn layer(&self, inner: S) -> Self::Service {
        ValidationMiddleware { inner }
    }
}

#[derive(Clone)]
pub struct ValidationMiddleware<S> {
    inner: S,
}

impl<S> tower::Service<Request> for ValidationMiddleware<S>
where
    S: tower::Service<Request, Response = Response> + Clone + Send + 'static,
    S::Future: Send + 'static,
    S::Error: std::error::Error + Send + Sync + 'static,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: Request) -> Self::Future {
        let mut inner = self.inner.clone();

        Box::pin(async move {
            if let Err(_e) = validation::validate_request(&req) {
                let resp = Response::builder()
                    .status(axum::http::StatusCode::BAD_REQUEST)
                    .header("content-type", "application/json")
                    .body(axum::body::Body::from(
                        r#"{"success":false,"message":"Validation failed"}"#,
                    ))
                    .expect("failed to build validation error response");
                return Ok(resp);
            }

            inner.call(req).await
        })
    }
}
