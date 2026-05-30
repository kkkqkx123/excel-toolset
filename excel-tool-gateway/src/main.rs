#![allow(dead_code)]

mod cell_ref;
mod cli;
mod excel_data;
mod excel_diff;
mod excel_read;
mod excel_write;
mod file_util;
#[cfg(feature = "http")]
pub mod http;
mod security;
pub mod types;
mod vba_util;

fn main() {
    let args: Vec<String> = std::env::args().collect();

    // HTTP mode: --serve or EXCEL_MODE=http
    let is_http = args.iter().any(|a| a == "--serve")
        || std::env::var("EXCEL_MODE").map(|v| v == "http").unwrap_or(false);

    if is_http {
        #[cfg(feature = "http")]
        {
            start_http_server();
        }
        #[cfg(not(feature = "http"))]
        {
            eprintln!("HTTP server not available. Build with --features http");
            std::process::exit(1);
        }
    } else {
        #[cfg(feature = "cli")]
        {
            run_cli();
        }
        #[cfg(not(feature = "cli"))]
        {
            eprintln!("CLI not available. Build with --features cli (default)");
            std::process::exit(1);
        }
    }
}

#[cfg(feature = "cli")]
fn run_cli() {
    use clap::Parser;
    let cli = cli::Cli::parse();
    cli::execute(&cli);
}

#[cfg(feature = "http")]
fn start_http_server() {
    use tokio::runtime::Runtime;

    let rt = Runtime::new().expect("Failed to create tokio runtime");
    rt.block_on(async {
        let app = http::router::create_router();
        let listener = tokio::net::TcpListener::bind("0.0.0.0:3000")
            .await
            .expect("Failed to bind");
        println!("HTTP server listening on http://0.0.0.0:3000");
        axum::serve(listener, app).await.expect("Server error");
    });
}

#[cfg(test)]
mod tests {
    use crate::types::*;

    #[test]
    fn test_api_response() {
        let resp: ApiResponse<String> = ApiResponse {
            success: true,
            message: "ok".into(),
            file_hash: None,
            data: None,
            diff: None,
            backup_info: None,
        };
        assert!(resp.success);
    }
}
