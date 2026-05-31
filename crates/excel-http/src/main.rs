mod http;

#[tokio::main]
async fn main() {
    let app = http::router::create_router();
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000")
        .await
        .unwrap_or_else(|e| {
            eprintln!("Failed to bind to 0.0.0.0:3000: {}", e);
            std::process::exit(1);
        });
    println!("HTTP server listening on http://0.0.0.0:3000");
    if let Err(e) = axum::serve(listener, app).await {
        eprintln!("Server error: {}", e);
        std::process::exit(1);
    }
}
