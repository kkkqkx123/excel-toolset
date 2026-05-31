use excel_core::types::AppError;

pub type SqlResult<T> = std::result::Result<T, AppError>;
