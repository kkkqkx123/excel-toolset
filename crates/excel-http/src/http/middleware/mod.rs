pub mod validation;

use axum::{extract::Request, http::StatusCode, response::Response};
use tower::ServiceExt;

pub struct ValidationLayer;

impl<S> tower::Layer<S> for ValidationLayer {
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
    S: tower::Service<Request> + Clone,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = tower::util::BoxFuture<'static, Result<Self::Response, Self::Error>>;

    fn poll_ready(
        &mut self,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: Request) -> Self::Future {
        let mut inner = self.inner.clone();

        Box::pin(async move {
            if let Err(e) = validation::validate_request(&req) {
                return Ok(Response::builder()
                    .status(StatusCode::BAD_REQUEST)
                    .header("content-type", "application/json")
                    .body(axum::body::Body::from(serde_json::to_string(&e).unwrap()))
                    .unwrap());
            }

            inner.call(req).await
        })
    }
}