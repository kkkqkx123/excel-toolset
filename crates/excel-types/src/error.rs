use std::io;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("IO error: {0}")]
    Io(#[from] io::Error),

    #[error("Calamine error: {0}")]
    Calamine(#[from] calamine::XlsxError),

    #[error("Xlsx error: {0}")]
    Xlsx(#[from] rust_xlsxwriter::XlsxError),

    #[error("Sheet '{0}' not found")]
    SheetNotFound(String),

    #[error("Sheet '{0}' already exists")]
    SheetAlreadyExists(String),

    #[error("Cell ({0},{1}) not found")]
    CellNotFound(u32, u16),

    #[error("Invalid cell reference: {0}")]
    InvalidCellRef(String),

    #[error("Invalid range: {0}")]
    InvalidRange(String),

    #[error("Invalid argument: {0}")]
    InvalidArgument(String),

    #[error("Unknown filter operator: {0}")]
    InvalidFilterOp(String),

    #[error("Unknown chart type: {0}")]
    InvalidChartType(String),

    #[error("Unknown table style: {0}")]
    InvalidTableStyle(String),

    #[error("Unknown data validation type: {0}")]
    InvalidDataValidationType(String),

    #[error("Unknown pivot aggregation: {0}")]
    InvalidPivotAggregation(String),

    #[error("{0}")]
    VbaNotSupported(String),

    #[error("{0}")]
    FeatureNotEnabled(String),

    #[error("DuckDB error: {0}")]
    DuckDb(String),

    #[error("Serialization error: {0}")]
    Serialize(String),

    #[error("{0}")]
    Custom(String),

    #[error("Read error: {0}")]
    Read(String),

    #[error("Write error: {0}")]
    Write(String),

    #[error("Invalid input: {0}")]
    InvalidInput(String),
}

impl AppError {
    pub fn error_code(&self) -> &'static str {
        match self {
            AppError::Io(_) => "IO_ERROR",
            AppError::Calamine(_) => "CALAMINE_ERROR",
            AppError::Xlsx(_) => "XLSX_ERROR",
            AppError::SheetNotFound(_) => "SHEET_NOT_FOUND",
            AppError::SheetAlreadyExists(_) => "SHEET_ALREADY_EXISTS",
            AppError::CellNotFound(..) => "CELL_NOT_FOUND",
            AppError::InvalidCellRef(_) => "INVALID_CELL_REF",
            AppError::InvalidRange(_) => "INVALID_RANGE",
            AppError::InvalidArgument(_) => "INVALID_ARGUMENT",
            AppError::InvalidFilterOp(_) => "INVALID_FILTER_OP",
            AppError::InvalidChartType(_) => "INVALID_CHART_TYPE",
            AppError::InvalidTableStyle(_) => "INVALID_TABLE_STYLE",
            AppError::InvalidDataValidationType(_) => "INVALID_DATA_VALIDATION_TYPE",
            AppError::InvalidPivotAggregation(_) => "INVALID_PIVOT_AGGREGATION",
            AppError::VbaNotSupported(_) => "VBA_NOT_SUPPORTED",
            AppError::FeatureNotEnabled(_) => "FEATURE_NOT_ENABLED",
            AppError::DuckDb(_) => "DUCKDB_ERROR",
            AppError::Serialize(_) => "SERIALIZE_ERROR",
            AppError::Custom(_) => "CUSTOM_ERROR",
            AppError::Read(_) => "READ_ERROR",
            AppError::Write(_) => "WRITE_ERROR",
            AppError::InvalidInput(_) => "INVALID_INPUT",
        }
    }
}

pub type Result<T> = std::result::Result<T, AppError>;
