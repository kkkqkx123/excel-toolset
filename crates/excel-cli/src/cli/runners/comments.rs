use excel_core::features::comments;
use excel_core::types::*;
use crate::cli::args::*;

pub(crate) fn run_comments(args: &CommentsArgs) -> Result<serde_json::Value> {
    match &args.command {
        CommentsSub::Get { path, sheet, cell } => {
            let comment = comments::get_comment(path, sheet, cell)?;
            Ok(serde_json::to_value(comment).map_err(|e| AppError::Serialize(e.to_string()))?)
        }
        CommentsSub::Add {
            path,
            sheet,
            cell,
            text,
            dry_run,
        } => {
            let params = SecurityParams {
                dry_run: *dry_run,
                create_backup: true,
                file_path: path.clone(),
            };
            let result = comments::add_comment(path, sheet, cell, text, &params)?;
            Ok(serde_json::to_value(result).map_err(|e| AppError::Serialize(e.to_string()))?)
        }
        CommentsSub::Update {
            path,
            sheet,
            cell,
            text,
            dry_run,
        } => {
            let params = SecurityParams {
                dry_run: *dry_run,
                create_backup: true,
                file_path: path.clone(),
            };
            let result = comments::update_comment(path, sheet, cell, text, &params)?;
            Ok(serde_json::to_value(result).map_err(|e| AppError::Serialize(e.to_string()))?)
        }
        CommentsSub::Delete {
            path,
            sheet,
            cell,
            dry_run,
        } => {
            let params = SecurityParams {
                dry_run: *dry_run,
                create_backup: true,
                file_path: path.clone(),
            };
            let result = comments::delete_comment(path, sheet, cell, &params)?;
            Ok(serde_json::to_value(result).map_err(|e| AppError::Serialize(e.to_string()))?)
        }
    }
}

// ── Named Range ──

