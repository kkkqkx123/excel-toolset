mod http;

#[tokio::main]
async fn main() {
    let app = http::router::create_router();
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000")
        .await
        .expect("Failed to bind");
    println!("HTTP server listening on http://0.0.0.0:3000");
    axum::serve(listener, app).await.expect("Server error");
}
