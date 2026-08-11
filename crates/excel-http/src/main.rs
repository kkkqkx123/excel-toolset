mod http;

#[tokio::main]
async fn main() {
    let app = http::router::create_router();
    let port = std::env::var("PORT").unwrap_or_else(|_| "3000".to_string());
    // Bind to loopback by default. This server exposes unauthenticated
    // filesystem read/write endpoints, so listening on 0.0.0.0 handed the whole
    // filesystem to the local network. Opt in explicitly via EXCEL_HTTP_HOST.
    let host = std::env::var("EXCEL_HTTP_HOST").unwrap_or_else(|_| "127.0.0.1".to_string());
    let loopback = host == "127.0.0.1" || host == "localhost" || host == "::1";
    let has_token = http::middleware::guard::configured_token().is_some();
    let root = http::middleware::guard::configured_root();
    if !loopback && (!has_token || root.is_none()) {
        eprintln!(
            "WARNING: binding to {host} exposes file read/write operations beyond \
             this machine. Set EXCEL_HTTP_TOKEN (auth) and EXCEL_HTTP_ROOT (path \
             sandbox) before doing so."
        );
    }
    match &root {
        Some(r) => println!("File access restricted to: {}", r.display()),
        None => println!(
            "File access is UNRESTRICTED (set EXCEL_HTTP_ROOT to sandbox it)"
        ),
    }
    println!(
        "Authentication: {}",
        if has_token {
            "enabled (Bearer token)"
        } else {
            "disabled (set EXCEL_HTTP_TOKEN to enable)"
        }
    );
    let addr = format!("{}:{}", host, port);
    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .unwrap_or_else(|e| {
            eprintln!("Failed to bind to {}: {}", addr, e);
            std::process::exit(1);
        });
    println!("HTTP server listening on http://{}", addr);
    if let Err(e) = axum::serve(listener, app).await {
        eprintln!("Server error: {}", e);
        std::process::exit(1);
    }
}
