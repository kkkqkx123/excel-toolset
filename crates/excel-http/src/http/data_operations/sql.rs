use axum::{Json, extract::Path};
use serde::{Deserialize, Serialize};
#[cfg(feature = "sql")]
use std::collections::HashMap;
#[cfg(feature = "sql")]
use std::sync::atomic::{AtomicU64, Ordering};

#[cfg(feature = "sql")]
use excel_core::operations;
use excel_core::types::*;

#[derive(Deserialize)]
pub struct SqlReq {
    pub path: String,
    pub sheet: String,
    pub query: String,
    #[serde(default)]
    pub session_id: Option<String>,
    #[serde(default)]
    pub cache: bool,
}

#[derive(Deserialize)]
pub struct CreateSessionReq {
    pub path: String,
}

#[derive(Serialize)]
pub struct CreateSessionResp {
    pub session_id: String,
    pub tables: Vec<String>,
}

/// Global query cache available when the `sql` feature is enabled.
#[cfg(feature = "sql")]
static GLOBAL_CACHE: std::sync::LazyLock<std::sync::Mutex<excel_sql::QueryCache>> =
    std::sync::LazyLock::new(|| {
        std::sync::Mutex::new(excel_sql::QueryCache::new(
            excel_sql::QueryCacheConfig::default(),
        ))
    });

/// Global session pool
#[cfg(feature = "sql")]
static GLOBAL_SESSIONS: std::sync::LazyLock<
    std::sync::Mutex<HashMap<String, excel_sql::QuerySession>>,
> = std::sync::LazyLock::new(|| std::sync::Mutex::new(HashMap::new()));

/// Atomic counter for generating session IDs.
#[cfg(feature = "sql")]
static SESSION_COUNTER: AtomicU64 = AtomicU64::new(1);

pub async fn data_sql(Json(req): Json<SqlReq>) -> Json<ApiResponse<Vec<Vec<CellData>>>> {
    #[cfg(feature = "sql")]
    {
        // If session_id is provided, use session query
        if let Some(ref sid) = req.session_id {
            let sessions = GLOBAL_SESSIONS
                .lock()
                .expect("global SQL session lock poisoned");
            if let Some(session) = sessions.get(sid) {
                match session.query(&req.query) {
                    Ok(result) => {
                        return Json(ApiResponse::ok(Some(result.rows)));
                    }
                    Err(e) => return Json(ApiResponse::err(e)),
                }
            }
            return Json(ApiResponse::err(AppError::SheetNotFound(format!(
                "Session not found: {}",
                sid
            ))));
        }

        if req.cache {
            let key = excel_sql::QueryCache::make_key(&req.path, &req.query);
            {
                let mut cache = GLOBAL_CACHE.lock().expect("global SQL cache lock poisoned");
                if let Some(cached) = cache.get(&key) {
                    return Json(ApiResponse::ok(Some(cached.rows.clone())));
                }
            }
            match operations::sql_query(&req.path, &req.sheet, &req.query) {
                Ok(data) => {
                    let mut cache = GLOBAL_CACHE.lock().expect("global SQL cache lock poisoned");
                    cache.put(
                        key,
                        excel_sql::QueryResult {
                            columns: Vec::new(),
                            rows: data.clone(),
                            row_count: data.len(),
                        },
                    );
                    Json(ApiResponse::ok(Some(data)))
                }
                Err(e) => Json(ApiResponse::err(e)),
            }
        } else {
            match operations::sql_query(&req.path, &req.sheet, &req.query) {
                Ok(data) => Json(ApiResponse::ok(Some(data))),
                Err(e) => Json(ApiResponse::err(e)),
            }
        }
    }
    #[cfg(not(feature = "sql"))]
    {
        let _ = req;
        Json(ApiResponse::err(AppError::FeatureNotEnabled(
            "SQL queries require the 'sql' feature".into(),
        )))
    }
}

pub async fn create_session(
    Json(req): Json<CreateSessionReq>,
) -> Json<ApiResponse<CreateSessionResp>> {
    #[cfg(feature = "sql")]
    {
        let session_id = format!("sess-{}", SESSION_COUNTER.fetch_add(1, Ordering::SeqCst));
        let mut qs = match excel_sql::QuerySession::new() {
            Ok(s) => s,
            Err(e) => return Json(ApiResponse::err(e)),
        };
        if let Err(e) = qs.open_workbook(&req.path) {
            return Json(ApiResponse::err(e));
        }
        let tables = qs.list_tables().unwrap_or_default();
        let mut sessions = GLOBAL_SESSIONS
            .lock()
            .expect("global SQL session lock poisoned");
        sessions.insert(session_id.clone(), qs);
        Json(ApiResponse::ok(Some(CreateSessionResp {
            session_id,
            tables,
        })))
    }
    #[cfg(not(feature = "sql"))]
    {
        let _ = req;
        Json(ApiResponse::err(AppError::FeatureNotEnabled(
            "SQL sessions require the 'sql' feature".into(),
        )))
    }
}

pub async fn close_session(Path(id): Path<String>) -> Json<ApiResponse<String>> {
    #[cfg(feature = "sql")]
    {
        let mut sessions = GLOBAL_SESSIONS
            .lock()
            .expect("global SQL session lock poisoned");
        if sessions.remove(&id).is_some() {
            Json(ApiResponse::ok(Some(format!("Session {} closed", id))))
        } else {
            Json(ApiResponse::err(AppError::SheetNotFound(format!(
                "Session not found: {}",
                id
            ))))
        }
    }
    #[cfg(not(feature = "sql"))]
    {
        let _ = id;
        Json(ApiResponse::err(AppError::FeatureNotEnabled(
            "SQL sessions require the 'sql' feature".into(),
        )))
    }
}
