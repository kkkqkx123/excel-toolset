use std::fs;
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};
use std::time::Duration;

static TEST_COUNTER: AtomicU64 = AtomicU64::new(1);
static NEXT_PORT: AtomicU64 = AtomicU64::new(14000);
static SERVER_MUTEX: Mutex<()> = Mutex::new(());

fn ensure_binaries_built() {
    static BUILT: OnceLock<()> = OnceLock::new();
    BUILT.get_or_init(|| {
        let status = Command::new("cargo")
            .args(["build", "-p", "excel-cli", "-p", "excel-http"])
            .status()
            .expect("Failed to run cargo build for test binaries");
        assert!(
            status.success(),
            "cargo build -p excel-cli -p excel-http failed before tests"
        );
    });
}

fn test_id() -> u64 {
    TEST_COUNTER.fetch_add(1, Ordering::SeqCst)
}

fn next_port() -> u16 {
    NEXT_PORT.fetch_add(1, Ordering::SeqCst) as u16
}

fn cli_binary() -> std::path::PathBuf {
    ensure_binaries_built();
    let mut path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.pop();
    path.pop();
    path.push("target");
    path.push("debug");
    path.push("excel-cli");
    path
}

fn http_binary() -> std::path::PathBuf {
    let mut path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.pop();
    path.pop();
    path.push("target");
    path.push("debug");
    path.push("excel-http");
    path
}

fn test_dir(id: u64) -> std::path::PathBuf {
    let mut path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push("target");
    path.push("test-http-tmp");
    path.push(format!("t{:04}", id));
    fs::create_dir_all(&path).expect("create test dir");
    path
}

fn tf(id: u64, name: &str) -> String {
    let mut p = test_dir(id);
    p.push(name);
    p.to_string_lossy().to_string()
}

fn cli_run(args: &[&str]) -> serde_json::Value {
    let output = Command::new(cli_binary())
        .args(args)
        .output()
        .expect("CLI binary required (run `cargo build -p excel-cli` first)");
    let stdout = String::from_utf8_lossy(&output.stdout);
    serde_json::from_str(&stdout).unwrap_or_else(|_| {
        let stderr = String::from_utf8_lossy(&output.stderr);
        panic!("Bad JSON output:\nstderr: {}\nstdout: {}", stderr, stdout)
    })
}

fn assert_ok(json: &serde_json::Value) {
    if let Some(s) = json.get("success").and_then(|v| v.as_bool()) {
        assert!(s, "Command failed: {}", json);
    }
}

// =======================================================================
// HTTP Server management
// =======================================================================

struct TestServer {
    child: Child,
    port: u16,
    _guard: std::sync::MutexGuard<'static, ()>,
}

fn server_start_timeout() -> Duration {
    let ms = std::env::var("E2E_SERVER_START_TIMEOUT_MS")
        .ok()
        .and_then(|v| v.parse::<u64>().ok())
        .unwrap_or(6000);
    Duration::from_millis(ms)
}

fn server_poll_interval() -> Duration {
    Duration::from_millis(200)
}

impl TestServer {
    fn start() -> Self {
        let guard = SERVER_MUTEX.lock().expect("server mutex");
        let port = next_port();
        let binary = http_binary();
        assert!(
            binary.exists(),
            "excel-http binary not found at {:?}",
            binary
        );

        let mut child = Command::new(&binary)
            .env("PORT", port.to_string())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("Failed to start excel-http");

        // Wait for server to be ready
        let client = reqwest::blocking::Client::new();
        let health_url = format!("http://127.0.0.1:{}/health", port);
        let deadline = std::time::Instant::now() + server_start_timeout();
        loop {
            if client
                .get(&health_url)
                .timeout(Duration::from_secs(1))
                .send()
                .is_ok()
            {
                return TestServer {
                    child,
                    port,
                    _guard: guard,
                };
            }
            if std::time::Instant::now() > deadline {
                break;
            }
            std::thread::sleep(server_poll_interval());
        }

        // Try to read stderr for diagnostics
        let _ = child.kill();
        let _ = child.wait();
        let timeout_ms = server_start_timeout().as_millis();
        panic!(
            "Server failed to start on port {} within {}ms (set E2E_SERVER_START_TIMEOUT_MS to increase)",
            port, timeout_ms
        );
    }

    fn base_url(&self) -> String {
        format!("http://127.0.0.1:{}", self.port)
    }
}

impl Drop for TestServer {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

fn client() -> reqwest::blocking::Client {
    reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
        .expect("build client")
}

// =======================================================================
// Business Scenario 1: Health check
// Real scenario: Monitoring system checks if the service is alive.
// =======================================================================

#[test]
fn scenario_health_check() {
    let server = TestServer::start();
    let cli = client();
    let resp = cli
        .get(format!("{}/health", server.base_url()))
        .send()
        .expect("health request");
    assert!(resp.status().is_success());
    let body: serde_json::Value = resp.json().expect("json body");
    assert_eq!(body["status"], "ok");
}

// =======================================================================
// Business Scenario 2: Create file, write cell, read cell
// Real scenario: User creates a file via API, writes data, reads it back.
// =======================================================================

#[test]
fn scenario_create_write_read() {
    let server = TestServer::start();
    let cli = client();
    let id = test_id();
    let path = tf(id, "test.xlsx");
    let base = server.base_url();

    // Create file
    let resp = cli
        .post(format!("{}/api/file/create", base))
        .json(&serde_json::json!({
            "path": path,
            "sheet": "Data"
        }))
        .send()
        .expect("create file");
    assert!(resp.status().is_success());
    let body: serde_json::Value = resp.json().expect("json");
    assert!(body["success"].as_bool().unwrap());

    // Write cell
    let resp = cli
        .post(format!("{}/api/cell/write", base))
        .json(&serde_json::json!({
            "path": path,
            "sheet": "Data",
            "cell": "A1",
            "value": "Hello World",
            "dry_run": false
        }))
        .send()
        .expect("write cell");
    assert!(resp.status().is_success());

    // Read cell
    let resp = cli
        .post(format!("{}/api/cell/read", base))
        .json(&serde_json::json!({
            "path": path,
            "sheet": "Data",
            "cell": "A1"
        }))
        .send()
        .expect("read cell");
    let body: serde_json::Value = resp.json().expect("json");
    assert!(body["success"].as_bool().unwrap());
    assert_eq!(body["data"]["value"], "Hello World");

    let _ = fs::remove_file(&path);
}

// =======================================================================
// Business Scenario 3: Write range and read range
// Real scenario: User uploads a grid of data and retrieves it.
// =======================================================================

#[test]
fn scenario_range_write_and_read() {
    let server = TestServer::start();
    let cli = client();
    let id = test_id();
    let path = tf(id, "range.xlsx");
    let base = server.base_url();

    // Create file
    cli.post(format!("{}/api/file/create", base))
        .json(&serde_json::json!({"path": path, "sheet": "Grid"}))
        .send()
        .expect("create");

    // Write range
    let resp = cli
        .post(format!("{}/api/range/write", base))
        .json(&serde_json::json!({
            "path": path,
            "sheet": "Grid",
            "range": "A1:B2",
            "data": [
                ["Name", "Score"],
                ["Alice", "95"]
            ],
            "dry_run": false
        }))
        .send()
        .expect("write range");
    assert!(resp.status().is_success());

    // Read range
    let resp = cli
        .post(format!("{}/api/range/read", base))
        .json(&serde_json::json!({
            "path": path,
            "sheet": "Grid",
            "range": "A1:B2"
        }))
        .send()
        .expect("read range");
    let body: serde_json::Value = resp.json().expect("json");
    assert!(body["success"].as_bool().unwrap());
    let data = body["data"].as_array().expect("data array");
    assert_eq!(data.len(), 2);
    assert_eq!(data[0][0]["value"], "Name");

    let _ = fs::remove_file(&path);
}

// =======================================================================
// Business Scenario 4: File info
// Real scenario: User checks metadata of an Excel file.
// =======================================================================

#[test]
fn scenario_file_info() {
    let server = TestServer::start();
    let cli = client();
    let id = test_id();
    let path = tf(id, "info.xlsx");
    let base = server.base_url();

    // Create file using CLI for simplicity
    cli_run(&["file", "create", &path, "--sheet", "Sheet1"]);

    let resp = cli
        .post(format!("{}/api/file/info", base))
        .json(&serde_json::json!({"path": path}))
        .send()
        .expect("file info");
    let body: serde_json::Value = resp.json().expect("json");
    assert!(body["success"].as_bool().unwrap());
    assert!(!body["data"]["hash"].as_str().unwrap().is_empty());
    assert!(body["data"]["size"].as_u64().unwrap() > 0);

    let _ = fs::remove_file(&path);
}

// =======================================================================
// Business Scenario 5: Sheet management via API
// Real scenario: User adds and lists sheets.
// =======================================================================

#[test]
fn scenario_sheet_management_http() {
    let server = TestServer::start();
    let cli = client();
    let id = test_id();
    let path = tf(id, "sheets.xlsx");
    let base = server.base_url();

    // Create file
    cli.post(format!("{}/api/file/create", base))
        .json(&serde_json::json!({"path": path, "sheet": "Sheet1"}))
        .send()
        .expect("create");

    // Add sheet
    cli.post(format!("{}/api/sheet/add", base))
        .json(&serde_json::json!({"path": path, "name": "Extra"}))
        .send()
        .expect("add sheet");

    // List sheets
    let resp = cli
        .post(format!("{}/api/sheet/list", base))
        .json(&serde_json::json!({"path": path}))
        .send()
        .expect("list sheets");
    let body: serde_json::Value = resp.json().expect("json");
    let sheets = body["data"].as_array().expect("sheets array");
    assert!(sheets.iter().any(|s| s == "Sheet1"));
    assert!(sheets.iter().any(|s| s == "Extra"));

    let _ = fs::remove_file(&path);
}

// =======================================================================
// Business Scenario 6: Search via API
// Real scenario: User searches for data within a workbook.
// =======================================================================

#[test]
fn scenario_search_http() {
    let server = TestServer::start();
    let cli = client();
    let id = test_id();
    let path = tf(id, "search.xlsx");
    let base = server.base_url();

    // Create file with data using CLI
    cli_run(&["file", "create", &path, "--sheet", "Data"]);
    cli_run(&["cell", "write", &path, "Data", "A1", "Hello"]);
    cli_run(&["cell", "write", &path, "Data", "B1", "World"]);

    let resp = cli
        .post(format!("{}/api/search/workbook", base))
        .json(&serde_json::json!({
            "path": path,
            "pattern": "Hello",
            "search_type": "value",
            "match_type": "exact"
        }))
        .send()
        .expect("search");
    let body: serde_json::Value = resp.json().expect("json");
    assert!(body["success"].as_bool().unwrap());
    assert!(body["data"]["total_matches"].as_u64().unwrap() >= 1);

    let _ = fs::remove_file(&path);
}

// =======================================================================
// Business Scenario 7: Comments via API
// Real scenario: User adds and reads cell comments.
// =======================================================================

#[test]
fn scenario_comments_http() {
    let server = TestServer::start();
    let cli = client();
    let id = test_id();
    let path = tf(id, "comments.xlsx");
    let base = server.base_url();

    // Create file
    cli_run(&["file", "create", &path, "--sheet", "Sheet1"]);

    // Add comment
    let resp = cli
        .post(format!("{}/api/comments/add", base))
        .json(&serde_json::json!({
            "path": path,
            "sheet": "Sheet1",
            "cell": "A1",
            "comment": "Important note"
        }))
        .send()
        .expect("add comment");
    let body: serde_json::Value = resp.json().expect("json");
    assert!(body["success"].as_bool().unwrap());

    // Get comment
    let resp = cli
        .post(format!("{}/api/comments/get", base))
        .json(&serde_json::json!({
            "path": path,
            "sheet": "Sheet1",
            "cell": "A1"
        }))
        .send()
        .expect("get comment");
    let body: serde_json::Value = resp.json().expect("json");
    assert!(body["success"].as_bool().unwrap());
    assert_eq!(body["data"]["text"], "Important note");

    let _ = fs::remove_file(&path);
    let _ = fs::remove_file(format!("{}.comments.json", path));
}

// =======================================================================
// Business Scenario 8: Filter data via API
// Real scenario: User filters rows by condition.
// =======================================================================

#[test]
fn scenario_filter_http() {
    let server = TestServer::start();
    let cli = client();
    let id = test_id();
    let path = tf(id, "filter.xlsx");
    let base = server.base_url();

    // Create file with data
    cli_run(&["file", "create", &path, "--sheet", "Data"]);
    cli_run(&["cell", "write", &path, "Data", "A1", "Name"]);
    cli_run(&["cell", "write", &path, "Data", "B1", "Age"]);
    cli_run(&["cell", "write", &path, "Data", "A2", "Alice"]);
    cli_run(&["cell", "write", &path, "Data", "B2", "25"]);
    cli_run(&["cell", "write", &path, "Data", "A3", "Bob"]);
    cli_run(&["cell", "write", &path, "Data", "B3", "35"]);

    let resp = cli
        .post(format!("{}/api/data/filter", base))
        .json(&serde_json::json!({
            "path": path,
            "sheet": "Data",
            "column": 1,
            "operator": "Gt",
            "value": "30"
        }))
        .send()
        .expect("filter");
    let body: serde_json::Value = resp.json().expect("json");
    assert!(body["success"].as_bool().unwrap());
    let rows = body["data"].as_array().expect("rows array");
    // header + Bob
    assert!(rows.len() >= 2);

    let _ = fs::remove_file(&path);
}

// =======================================================================
// Business Scenario 9: File diff via API
// Real scenario: User compares two file versions.
// =======================================================================

#[test]
fn scenario_diff_http() {
    let server = TestServer::start();
    let cli = client();
    let id = test_id();
    let path1 = tf(id, "v1.xlsx");
    let path2 = tf(id, "v2.xlsx");
    let base = server.base_url();

    // Create two versions
    cli_run(&["file", "create", &path1, "--sheet", "Data"]);
    cli_run(&["cell", "write", &path1, "Data", "A1", "Version 1"]);

    cli_run(&["file", "create", &path2, "--sheet", "Data"]);
    cli_run(&["cell", "write", &path2, "Data", "A1", "Version 2"]);

    let resp = cli
        .post(format!("{}/api/diff/file", base))
        .json(&serde_json::json!({
            "old_path": path1,
            "new_path": path2
        }))
        .send()
        .expect("diff");
    let body: serde_json::Value = resp.json().expect("json");
    assert!(body["success"].as_bool().unwrap());
    assert!(body["data"]["summary"]["total_changes"].as_u64().unwrap() >= 1);

    let _ = fs::remove_file(&path1);
    let _ = fs::remove_file(&path2);
}

// =======================================================================
// Business Scenario 10: Batch modify via API
// Real scenario: User applies multiple changes atomically.
// =======================================================================

#[test]
fn scenario_batch_modify_http() {
    let server = TestServer::start();
    let cli = client();
    let id = test_id();
    let path = tf(id, "batch.xlsx");
    let base = server.base_url();

    // Create file
    cli.post(format!("{}/api/file/create", base))
        .json(&serde_json::json!({"path": path, "sheet": "Data"}))
        .send()
        .expect("create");

    // Batch operations
    let resp = cli
        .post(format!("{}/api/batch/modify", base))
        .json(&serde_json::json!({
            "path": path,
            "operations": [
                {"type": "WriteCell", "sheet": "Data", "row": 0, "col": 0, "value": "Name"},
                {"type": "WriteCell", "sheet": "Data", "row": 1, "col": 0, "value": "Alice"}
            ]
        }))
        .send()
        .expect("batch modify");
    let body: serde_json::Value = resp.json().expect("json");
    assert!(body["success"].as_bool().unwrap());

    // Verify
    let resp = cli
        .post(format!("{}/api/cell/read", base))
        .json(&serde_json::json!({
            "path": path,
            "sheet": "Data",
            "cell": "A1"
        }))
        .send()
        .expect("read");
    let body: serde_json::Value = resp.json().expect("json");
    assert_eq!(body["data"]["value"], "Name");

    let _ = fs::remove_file(&path);
}
