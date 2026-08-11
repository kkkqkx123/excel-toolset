# HTTP 模块修复方案：H1 / H2 / H3 / H4 / H5

**模块**：`excel-http`（crate）+ `excel-types`（信封定义）。发布版二进制当前**启动即 panic**，86 端点全不可达。

---

## H1 — 路由 `:id` 启动 panic（阻断级，已随 B3 修复）

### 根因
`crates/excel-http/src/http/router.rs:59` 使用 axum 0.7 语法 `:id`，但 `Cargo.toml` 锁定 `axum = "0.8"`。`Router::route()` 阶段直接拒绝：`Path segments must not start with ':'`。

### 修改方案（已应用，见 [01-构建修复](./01-构建修复-B1B2B3.md) B3）
```rust
.route("/api/data/sql_session/{id}", delete(sql::close_session))
```

---

## H2 — 响应信封与文档不符

### 现象
文档约定 `{success, data, error:{code,message}}`（成功 `error:null`；失败 `error:{code,message}`，且 `data`/`error` **始终存在**）。实际是平铺 `message`/`error_code`，且 `data` 在 `None` 时被 `skip_serializing_if` 省略（T8 多处）。

### 根因
```rust
// crates/excel-types/src/response.rs:7
pub struct ApiResponse<T: Serialize> {
    pub success: bool,
    pub message: String,                       // 平铺
    pub error_code: Option<&'static str>,      // 平铺，无嵌套 error
    pub data: Option<T>,                       // skip_serializing_if → 成功时可能缺 data 键
    ...
}
```

### 修改方案（对齐文档信封）
新增嵌套错误结构，并使 `data`/`error` **始终序列化**（精确匹配文档）：
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiErrorDetail {
    pub code: &'static str,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiResponse<T: Serialize> {
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,          // 可选，兼容现有用法
    pub data: Option<T>,                  // 不再 skip → 失败时为 null，始终存在
    pub error: Option<ApiErrorDetail>,    // 嵌套错误；成功时为 null
    #[serde(skip_serializing_if = "Option::is_none")]
    pub diff: Option<FileDiff>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub backup_info: Option<BackupInfo>,
}

impl<T: Serialize> ApiResponse<T> {
    pub fn ok(data: Option<T>) -> Self {
        Self { success: true, message: None, data, error: None, diff: None, backup_info: None }
    }
    pub fn err(e: AppError) -> Self {
        let code = e.error_code();
        Self {
            success: false,
            message: Some(e.to_string()),
            data: None,
            error: Some(ApiErrorDetail { code, message: e.to_string() }),
            diff: None, backup_info: None,
        }
    }
}
```
> 这是**破坏性线格式变更**，但方向与文档契约一致（报告将 docs 视为权威）。需同步更新 `skills/excel-http` 中任何依赖旧平铺字段的示例（见 [07-文档校订](./07-文档校订-P3.md)）。

---

## H3 — 失败操作仍返回 HTTP 200

### 现象
文件不存在等业务失败一律 `200 OK`，只在 body 写 `success:false`（T8.65）。任何按状态码判断的通用 HTTP 客户端都会误判成功。

### 根因
所有 handler 返回 `Json(ApiResponse::err(e))`，而 `Json(...)` 默认 `StatusCode::OK`。

### 修改方案（引入类型化错误响应，axum 惯用法）
```rust
// 在 excel-types/src/response.rs 或 excel-http 内新增
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};

pub struct ApiErrorResponse {
    pub status: StatusCode,
    pub body: ApiResponse<()>,
}
impl IntoResponse for ApiErrorResponse {
    fn into_response(self) -> Response {
        (self.status, Json(self.body)).into_response()
    }
}
impl From<AppError> for ApiErrorResponse {
    fn from(e: AppError) -> Self {
        let status = status_for(&e);     // 见下
        ApiErrorResponse { status, body: ApiResponse::err(e) }
    }
}

pub type ApiResult<T> = Result<Json<ApiResponse<T>>, ApiErrorResponse>;

fn status_for(e: &AppError) -> StatusCode {
    match e {
        AppError::SheetNotFound(_) | AppError::InvalidInput(_) => StatusCode::BAD_REQUEST,
        AppError::Io(_) | AppError::NotFound(_) => StatusCode::NOT_FOUND,
        AppError::FeatureNotEnabled(_) => StatusCode::BAD_REQUEST,
        AppError::DuckDb(_) => StatusCode::BAD_REQUEST,
        _ => StatusCode::INTERNAL_SERVER_ERROR,
    }
}
```
**handler 改造模式**（机械、覆盖所有 `handlers/*.rs`）：
```rust
// 旧
pub async fn chart_create(Json(req): Json<ChartCreateReq>) -> Json<ApiResponse<WriteResult>> {
    ...
    Err(e) => return Json(ApiResponse::err(e)),
}
// 新
pub async fn chart_create(Json(req): Json<ChartCreateReq>) -> ApiResult<WriteResult> {
    ...
    match excel_write::add_chart(...) {
        Ok(data) => Ok(Json(ApiResponse::ok(Some(data)))),
        Err(e) => Err(e.into()),          // ← 自动带上正确状态码
    }
}
```
> 改造范围：`excel-http/src/http/handlers/**` 中返回 `Json(ApiResponse<...>)` 的全部函数。属机械替换，可由一次 PR 完成。

---

## H4 — 🔒 安全：监听 `0.0.0.0` + 任意路径写 + 无鉴权

### 现象
- 绑定 `0.0.0.0:3000`（文档说 `127.0.0.1`）（T8.03）。
- `POST /api/file/create {"path":"/tmp/anything.xlsx"}` 成功 → 可写文件系统任意位置（T8.69）。
- 无任何认证。

### 根因
```rust
// crates/excel-http/src/main.rs:7
let addr = format!("0.0.0.0:{}", port);
```

### 修改方案（三处叠加 = 未授权远程任意文件读写，必须一起修）

**1) 只绑回环（阻断远程访问）**
```rust
let addr = format!("127.0.0.1:{}", port);
```

**2) 路径白名单（阻断任意位置写）**
在 `excel-core/src/security.rs` 新增：
```rust
/// 拒绝绝对路径与 `..` 穿越；可选限制在允许根目录内
pub fn validate_path_inside_root(path: &str, root: &Path) -> Result<()> {
    let p = Path::new(path).canonicalize().map_err(|e| AppError::Io(e))?;
    if p.components().any(|c| matches!(c, Component::ParentDir)) {
        return Err(AppError::InvalidInput("path traversal not allowed".into()));
    }
    if !p.starts_with(root) {
        return Err(AppError::InvalidInput("path outside allowed root".into()));
    }
    Ok(())
}
```
在 `create_backup_if_needed`、各 `handlers` 打开/创建文件前调用（root 默认取进程 cwd，或由 `--root` / `EXCEL_HTTP_ROOT` 配置）。

**3) 鉴权（阻断未授权调用）**
新增 bearer-token 中间件：读取 `AUTHORIZATION` 头（或 `?token=`），与 `EXCEL_HTTP_TOKEN` 比对；缺失/不匹配返回 `401`。可复用 axum `middleware::from_fn`。至少应**默认要求 token**，并在 README/文档明确告知。

### 验证
- 从容器外 `curl http://<host>:3000/health` 不可达（仅 `127.0.0.1`）（T8.03 复测）。
- `POST /api/file/create {"path":"/tmp/x.xlsx"}` → `400`/`403`（T8.69 复测）。
- 无 token 请求 → `401`；带正确 token → 正常。

---

## H5 — `chart/create` 在 HTTP 端真实损坏

### 现象
CLI `chart create f.xlsx Data B1:B4 bar` 成功；相同参数走 HTTP `POST /api/chart/create {range:"B1:B4"}` → `Chart error: 'Chart series must contain a 'values' range'`（T9.19）。

### 根因
```rust
// crates/excel-http/src/http/handlers/chart.rs:39
let config = ChartConfig {
    ...
    categories_range: req.range.clone(),   // ← 与 values_range 完全相同
    values_range: req.range,               // ← rust_xlsxwriter 要求 values 为独立区间
    ...
};
```

### 修改方案（按 CLI 正确逻辑拆分区间）
参考 `excel-cli/src/cli/runners.rs` 中正确的图表区间处理：`categories` = 区间**第一列**，`values` = 剩余列。
```rust
use crate::utils::cell_ref::parse_range;   // 返回 (r1, c1, r2, c2)，0-based

// 在 chart_create 内，parse 之后：
let (r1, c1, r2, c2) = parse_range(&req.range)?;
let col_letter = |c: u16| -> String { index_to_col((c as usize).saturating_sub(1)) }; // 复用现有列名工具
let categories_range = format!("{}{}:{}{}", col_letter(c1), r1 + 1, col_letter(c1), r2 + 1);
let values_range = if c2 > c1 {
    format!("{}{}:{}{}", col_letter(c1 + 1), r1 + 1, col_letter(c2), r2 + 1)
} else {
    // 单列数据：values 退化为该列自身（与 CLI 行为对齐前需确认；更稳妥是要求 range 含 ≥2 列）
    categories_range.clone()
};
let config = ChartConfig {
    ...
    categories_range,
    values_range,
    ...
};
```
> `index_to_col` 在 `excel-diff/src/helpers.rs:88` 已实现（0-based→字母），`excel-core` 内若有同类工具直接复用，否则引入该辅助函数。若图表语义要求单列既作类别又作值，应在请求校验阶段给出清晰错误而非崩到 rust_xlsxwriter。

### 验证
- `POST /api/chart/create {sheet, range:"B1:B4", chart_type:"bar"}` → 成功且产物含有效 chart 部件（T9.19 复测）。
- 用 openpyxl 读取产物，`chart.series` 的 `values` 区间不为空。

---

## 工作量与依赖

| 项 | 改动量 | 风险 |
|----|--------|------|
| H1 | 1 字符（已修） | 无 |
| H2 | `response.rs` 结构改造 + 文档同步 | 中（线格式破坏性） |
| H3 | 新增 `ApiErrorResponse` + 全 handler 返回类型改造 | 中（范围大但机械） |
| H4 | `main.rs` 1 行 + `security.rs` 路径校验 + 鉴权中间件 | 中（安全敏感，需评审） |
| H5 | `chart.rs` ~12 行 | 低 |

**合计约 4–6 小时**（含编译 + t8/t9 套件复测）。H2/H3 建议作为**一次统一 PR** 提交，避免信封半改状态。
