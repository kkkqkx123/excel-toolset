use excel_types::AppError;

pub type SqlResult<T> = std::result::Result<T, AppError>;
