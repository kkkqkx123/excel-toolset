use std::io::Write;

use calamine::{open_workbook, Reader, Xlsx};

use crate::types::*;

pub fn has_vba(path: &str) -> Result<bool> {
    let mut workbook: Xlsx<_> =
        open_workbook(path).map_err(|e: calamine::XlsxError| AppError::Calamine(e.to_string()))?;
    Ok(workbook.vba_project().is_some())
}

/// Export VBA modules as a binary blob: (module count u32le) × [(name_len u32le, name, code_len u32le, code)]
pub fn export_vba(path: &str) -> Result<Vec<u8>> {
    let mut workbook: Xlsx<_> =
        open_workbook(path).map_err(|e: calamine::XlsxError| AppError::Calamine(e.to_string()))?;
    match workbook.vba_project() {
        Some(Ok(vba)) => {
            let mut buf = Vec::new();
            let names = vba.get_module_names();
            let count = names.len() as u32;
            buf.write_all(&count.to_le_bytes()).map_err(AppError::Io)?;
            for name in &names {
                let name_bytes = name.as_bytes();
                buf.write_all(&(name_bytes.len() as u32).to_le_bytes())
                    .map_err(AppError::Io)?;
                buf.write_all(name_bytes).map_err(AppError::Io)?;
                match vba.get_module_raw(name) {
                    Ok(code) => {
                        buf.write_all(&(code.len() as u32).to_le_bytes())
                            .map_err(AppError::Io)?;
                        buf.write_all(code).map_err(AppError::Io)?;
                    }
                    Err(_) => {
                        let zero = 0u32;
                        buf.write_all(&zero.to_le_bytes())
                            .map_err(AppError::Io)?;
                    }
                }
            }
            Ok(buf)
        }
        Some(Err(e)) => Err(AppError::Calamine(e.to_string())),
        None => Err(AppError::Custom("No VBA project found".into())),
    }
}

/// VBA import is not supported in rust_xlsxwriter 0.50.
pub fn import_vba(_path: &str, _params: &SecurityParams, _vba_data: &[u8]) -> Result<WriteResult> {
    Err(AppError::Custom(
        "VBA import is not supported in rust_xlsxwriter 0.50".into(),
    ))
}
