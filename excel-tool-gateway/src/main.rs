#![allow(dead_code)]

mod cell_ref;
mod cli;
mod excel_data;
mod excel_diff;
mod excel_read;
mod excel_write;
mod file_util;
pub mod http;
mod security;
pub mod types;
mod vba_util;

fn main() {
    println!("Excel Tool Gateway v0.1.0");
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
