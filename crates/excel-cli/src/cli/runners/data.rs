use excel_core::excel_write;
use excel_core::operations;
use excel_core::types::*;
use excel_core::utils::helpers;
use crate::cli::args::*;

pub(crate) fn run_data(args: &DataArgs) -> Result<serde_json::Value> {
    match &args.command {
        DataSub::AppendRow {
            path,
            sheet,
            values,
            dry_run,
        } => {
            let cell_values: Vec<Vec<CellValue>> = vec![
                values
                    .iter()
                    .map(|v| helpers::parse_cell_value(v))
                    .collect(),
            ];
            let params = SecurityParams {
                dry_run: *dry_run,
                create_backup: true,
                file_path: path.clone(),
            };
            let result = excel_write::append_rows(path, &params, sheet, &cell_values)?;
            Ok(serde_json::to_value(result).map_err(|e| AppError::Serialize(e.to_string()))?)
        }
        DataSub::InsertRow {
            path,
            sheet,
            row,
            values,
            dry_run,
        } => {
            let cell_values: Vec<Vec<CellValue>> = vec![
                values
                    .iter()
                    .map(|v| helpers::parse_cell_value(v))
                    .collect(),
            ];
            let params = SecurityParams {
                dry_run: *dry_run,
                create_backup: true,
                file_path: path.clone(),
            };
            // CLI row numbers are 1-indexed, internal functions use 0-indexed
            let row_idx = row.saturating_sub(1);
            let result = excel_write::insert_rows(path, &params, sheet, row_idx, &cell_values)?;
            Ok(serde_json::to_value(result).map_err(|e| AppError::Serialize(e.to_string()))?)
        }
        DataSub::DeleteRow {
            path,
            sheet,
            row,
            dry_run,
        } => {
            let params = SecurityParams {
                dry_run: *dry_run,
                create_backup: true,
                file_path: path.clone(),
            };
            // CLI row numbers are 1-indexed, internal functions use 0-indexed
            let row_idx = row.saturating_sub(1);
            let result = excel_write::delete_rows(path, &params, sheet, row_idx, row_idx)?;
            Ok(serde_json::to_value(result).map_err(|e| AppError::Serialize(e.to_string()))?)
        }
        DataSub::Filter {
            path,
            sheet,
            column,
            op,
            value,
        } => {
            let filter_op = helpers::parse_filter_op(op)?;
            let col_idx = column.saturating_sub(1);
            let conditions = vec![FilterCondition {
                column: col_idx,
                operator: filter_op,
                value: value.clone(),
            }];
            let result = operations::filter_rows(path, sheet, &conditions)?;
            Ok(serde_json::json!({
                "success": true,
                "rows": result
            }))
        }
        DataSub::Sort {
            path,
            sheet,
            column,
            desc,
            dry_run,
        } => {
            let col_idx = column.saturating_sub(1);
            let sort_cols = vec![SortColumn {
                column: col_idx,
                descending: *desc,
            }];
            let params = SecurityParams {
                dry_run: *dry_run,
                create_backup: true,
                file_path: path.clone(),
            };
            let result = operations::sort_sheet(path, &params, sheet, &sort_cols)?;
            Ok(serde_json::to_value(result).map_err(|e| AppError::Serialize(e.to_string()))?)
        }
        DataSub::Dedup {
            path,
            sheet,
            column,
            dry_run,
        } => {
            let cols: Vec<u16> = column
                .map(|c| vec![c.saturating_sub(1)])
                .unwrap_or_default();
            let params = SecurityParams {
                dry_run: *dry_run,
                create_backup: true,
                file_path: path.clone(),
            };
            let result = operations::dedup_sheet(path, &params, sheet, &cols)?;
            Ok(serde_json::to_value(result).map_err(|e| AppError::Serialize(e.to_string()))?)
        }
        DataSub::Sql {
            path,
            sheet,
            query,
            session,
            cache,
            no_header,
        } => {
            // Session support is implemented in Task 5 (QuerySession enhancement).
            #[cfg(feature = "sql")]
            {
                let has_header = !*no_header;
                if *cache {
                    let config = excel_sql::QueryCacheConfig::default();
                    let mut query_cache = excel_sql::QueryCache::new(config);
                    let key = excel_sql::QueryCache::make_key(path, query);
                    if let Some(cached) = query_cache.get(&key) {
                        return Ok(serde_json::to_value(cached)
                            .map_err(|e| AppError::Serialize(e.to_string()))?);
                    }
                    let result = operations::sql_query(path, sheet, query, has_header)?;
                    query_cache.put(
                        key,
                        excel_sql::QueryResult {
                            columns: Vec::new(),
                            rows: result.clone(),
                            row_count: result.len(),
                        },
                    );
                    return Ok(serde_json::to_value(result)
                        .map_err(|e| AppError::Serialize(e.to_string()))?);
                }
                if *session {
                    let mut qs = excel_sql::QuerySession::new()?;
                    qs.open_workbook(path)?;
                    let result = qs.query(query)?;
                    return Ok(serde_json::to_value(result)
                        .map_err(|e| AppError::Serialize(e.to_string()))?);
                }
                let result = operations::sql_query(path, sheet, query, has_header)?;
                Ok(serde_json::to_value(result).map_err(|e| AppError::Serialize(e.to_string()))?)
            }
            #[cfg(not(feature = "sql"))]
            {
                let _ = (path, sheet, query, session, cache, no_header);
                Err(AppError::FeatureNotEnabled(
                    "SQL queries require the 'sql' feature".into(),
                ))
            }
        }
    }
}

