use crate::cli::args::*;
use excel_core::excel_write;
use excel_core::types::*;

pub(crate) fn run_image(args: &ImageArgs) -> Result<serde_json::Value> {
    match &args.command {
        ImageSub::Insert {
            path,
            config,
            dry_run,
        } => {
            let image_config: ImageConfig = serde_json::from_str(config)
                .map_err(|e| AppError::Serialize(format!("Invalid image config JSON: {}", e)))?;
            let params = SecurityParams {
                dry_run: *dry_run,
                create_backup: true,
                file_path: path.clone(),
            };
            let result = excel_write::insert_image(path, &params, &image_config)?;
            Ok(serde_json::to_value(result).map_err(|e| AppError::Serialize(e.to_string()))?)
        }
        ImageSub::Remove {
            path,
            sheet,
            anchor_cell,
            dry_run,
        } => {
            let params = SecurityParams {
                dry_run: *dry_run,
                create_backup: true,
                file_path: path.clone(),
            };
            let result = excel_write::remove_image(path, &params, sheet, anchor_cell)?;
            Ok(serde_json::to_value(result).map_err(|e| AppError::Serialize(e.to_string()))?)
        }
        ImageSub::ShapeInsert {
            path,
            config,
            dry_run,
        } => {
            let shape_config: ShapeConfig = serde_json::from_str(config)
                .map_err(|e| AppError::Serialize(format!("Invalid shape config JSON: {}", e)))?;
            let params = SecurityParams {
                dry_run: *dry_run,
                create_backup: true,
                file_path: path.clone(),
            };
            let result = excel_write::insert_shape(path, &params, &shape_config)?;
            Ok(serde_json::to_value(result).map_err(|e| AppError::Serialize(e.to_string()))?)
        }
    }
}
