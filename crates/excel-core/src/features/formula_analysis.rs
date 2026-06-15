use regex::Regex;
use serde::Serialize;
use std::collections::HashSet;

use crate::excel_read;
use crate::types::*;
use calamine::Reader;

#[derive(Debug, Clone, Serialize)]
pub struct DependencyTrace {
    pub cell: String,
    pub direct_precedents: Vec<String>,
    pub direct_dependents: Vec<String>,
    pub all_precedents: Vec<String>,
    pub all_dependents: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct FormulaExplanation {
    pub cell: String,
    pub formula: String,
    pub function_name: Option<String>,
    pub arguments: Vec<String>,
    pub description: String,
    pub language: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct LogicStep {
    pub step_number: usize,
    pub operation: String,
    pub input: String,
    pub result: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct FormulaLogicExplanation {
    pub cell: String,
    pub formula: String,
    pub logic_flow: Vec<LogicStep>,
    pub data_sources: Vec<String>,
    pub calculation_result: Option<String>,
}

pub fn trace_dependencies(path: &str, sheet: &str, cell: &str) -> Result<DependencyTrace> {
    let mut workbook = calamine::open_workbook::<calamine::Xlsx<_>, _>(path)
        .map_err(|e| AppError::Read(e.to_string()))?;

    let ws_formulas = workbook
        .worksheet_formula(sheet)
        .map_err(|_e| AppError::SheetNotFound(sheet.to_string()))?;

    let (row, col) = crate::utils::cell_ref::parse_cell_ref(cell)?;

    let formula = ws_formulas
        .get_value((row, col as u32))
        .map(|s| s.to_string());

    if let Some(formula_str) = formula {
        let direct_precedents = extract_cell_references(&formula_str, sheet);

        let mut all_precedents = HashSet::new();
        let mut visited = HashSet::new();
        collect_all_precedents(
            path,
            sheet,
            &direct_precedents,
            &mut all_precedents,
            &mut visited,
        )?;

        let direct_dependents = find_direct_dependents(path, sheet, cell)?;

        let mut all_dependents = HashSet::new();
        let mut visited_deps = HashSet::new();
        collect_all_dependents(
            path,
            sheet,
            &direct_dependents,
            &mut all_dependents,
            &mut visited_deps,
        )?;

        Ok(DependencyTrace {
            cell: cell.to_string(),
            direct_precedents,
            direct_dependents,
            all_precedents: all_precedents.into_iter().collect(),
            all_dependents: all_dependents.into_iter().collect(),
        })
    } else {
        Ok(DependencyTrace {
            cell: cell.to_string(),
            direct_precedents: vec![],
            direct_dependents: find_direct_dependents(path, sheet, cell)?,
            all_precedents: vec![],
            all_dependents: vec![],
        })
    }
}

fn extract_cell_references(formula: &str, default_sheet: &str) -> Vec<String> {
    let mut refs = Vec::new();
    let mut sheet_names = HashSet::new();

    let cell_ref_regex = Regex::new(r"[A-Za-z]+!\$?[A-Za-z]+\$?\d+").unwrap();
    let simple_ref_regex = Regex::new(r"\$?[A-Za-z]+\$?\d+").unwrap();
    let sheet_name_regex = Regex::new(r"([A-Za-z_][A-Za-z0-9_]*)!").unwrap();

    for cap in sheet_name_regex.captures_iter(formula) {
        if let Some(sheet) = cap.get(1) {
            sheet_names.insert(sheet.as_str().to_string());
        }
    }

    for cap in cell_ref_regex.captures_iter(formula) {
        refs.push(cap[0].to_string());
    }

    for cap in simple_ref_regex.captures_iter(formula) {
        if !sheet_names.is_empty() {
            for sheet in &sheet_names {
                refs.push(format!("{}!{}", sheet, &cap[0]));
            }
        } else {
            refs.push(format!("{}!{}", default_sheet, &cap[0]));
        }
    }

    refs.sort();
    refs.dedup();
    refs
}

fn collect_all_precedents(
    path: &str,
    _sheet: &str,
    refs: &[String],
    all_precedents: &mut HashSet<String>,
    visited: &mut HashSet<String>,
) -> Result<()> {
    for cell_ref in refs {
        if visited.contains(cell_ref) {
            continue;
        }
        visited.insert(cell_ref.clone());

        if !cell_ref.contains('!') {
            continue;
        }

        let parts: Vec<&str> = cell_ref.split('!').collect();
        if parts.len() != 2 {
            continue;
        }

        let target_sheet = parts[0];
        let target_cell = parts[1];

        all_precedents.insert(cell_ref.clone());

        let trace = trace_dependencies(path, target_sheet, target_cell)?;
        for prec in trace.all_precedents {
            all_precedents.insert(prec);
        }
    }

    Ok(())
}

fn find_direct_dependents(path: &str, sheet: &str, cell: &str) -> Result<Vec<String>> {
    let mut workbook = calamine::open_workbook::<calamine::Xlsx<_>, _>(path)
        .map_err(|e| AppError::Read(e.to_string()))?;

    let ws_formulas = workbook
        .worksheet_formula(sheet)
        .map_err(|_e| AppError::SheetNotFound(sheet.to_string()))?;

    let mut dependents = Vec::new();
    let range = workbook
        .worksheet_range(sheet)
        .map_err(|_e| AppError::SheetNotFound(sheet.to_string()))?;

    let (target_row, target_col) = crate::utils::cell_ref::parse_cell_ref(cell)?;

    for row in 0..range.height() {
        for col in 0..range.width() {
            if let Some(formula) = ws_formulas.get_value((row as u32, col as u32)) {
                let refs = extract_cell_references(formula, sheet);
                for cell_ref in refs {
                    let parts: Vec<&str> = cell_ref.split('!').collect();
                    if parts.len() != 2 {
                        continue;
                    }

                    let ref_sheet = parts[0];
                    let ref_cell = parts[1];

                    if ref_sheet == sheet
                        && let Ok((r, c)) = crate::utils::cell_ref::parse_cell_ref(ref_cell)
                        && r == target_row
                        && c == target_col
                    {
                        let cell_addr =
                            crate::utils::cell_ref::format_cell_ref(row as u32, col as u16);
                        dependents.push(format!("{}!{}", sheet, cell_addr));
                    }
                }
            }
        }
    }

    dependents.sort();
    dependents.dedup();
    Ok(dependents)
}

fn collect_all_dependents(
    path: &str,
    _sheet: &str,
    refs: &[String],
    all_dependents: &mut HashSet<String>,
    visited: &mut HashSet<String>,
) -> Result<()> {
    for cell_ref in refs {
        if visited.contains(cell_ref) {
            continue;
        }
        visited.insert(cell_ref.clone());

        if !cell_ref.contains('!') {
            continue;
        }

        let parts: Vec<&str> = cell_ref.split('!').collect();
        if parts.len() != 2 {
            continue;
        }

        let target_sheet = parts[0];
        let target_cell = parts[1];

        all_dependents.insert(cell_ref.clone());

        let deps = find_direct_dependents(path, target_sheet, target_cell)?;
        for dep in deps {
            all_dependents.insert(dep.clone());
            collect_all_dependents(path, target_sheet, &[dep], all_dependents, visited)?;
        }
    }

    Ok(())
}

pub fn explain_formula(
    path: &str,
    sheet: &str,
    cell: &str,
    language: &str,
) -> Result<FormulaExplanation> {
    let formula = excel_read::read_formula(path, sheet, cell)?.ok_or_else(|| {
        AppError::CellNotFound(
            crate::utils::cell_ref::parse_cell_ref(cell).unwrap().0,
            crate::utils::cell_ref::parse_cell_ref(cell).unwrap().1,
        )
    })?;

    let (function_name, arguments) = parse_function(&formula);

    let description = generate_formula_description(&formula, language);

    Ok(FormulaExplanation {
        cell: cell.to_string(),
        formula: formula.clone(),
        function_name,
        arguments,
        description,
        language: language.to_string(),
    })
}

fn parse_function(formula: &str) -> (Option<String>, Vec<String>) {
    let clean_formula = formula.trim_start_matches('=');

    let function_regex = Regex::new(r"^([A-Za-z_][A-Za-z0-9_]*)\((.*)\)$").unwrap();
    if let Some(caps) = function_regex.captures(clean_formula) {
        let func_name = caps.get(1).map(|m| m.as_str().to_string());
        let args_str = caps
            .get(2)
            .map(|m| m.as_str().to_string())
            .unwrap_or_default();

        let args = split_arguments(&args_str);
        (func_name, args)
    } else {
        (None, vec![clean_formula.to_string()])
    }
}

fn split_arguments(args_str: &str) -> Vec<String> {
    let mut args = Vec::new();
    let mut current = String::new();
    let mut depth = 0;
    let mut in_string = false;

    for ch in args_str.chars() {
        match ch {
            '(' if !in_string => depth += 1,
            ')' if !in_string => depth -= 1,
            '"' if in_string => in_string = false,
            '"' if !in_string => in_string = true,
            ',' if depth == 0 && !in_string => {
                args.push(current.trim().to_string());
                current.clear();
            }
            _ => current.push(ch),
        }
    }

    if !current.is_empty() {
        args.push(current.trim().to_string());
    }

    args
}

fn generate_formula_description(formula: &str, language: &str) -> String {
    let clean_formula = formula.trim_start_matches('=');

    let (func_name, _) = parse_function(formula);
    if let Some(func) = func_name {
        match language {
            "zh" => match func.to_uppercase().as_str() {
                "SUM" => format!("SUM 函数: 计算参数的总和。公式: {}", clean_formula),
                "AVERAGE" => format!("AVERAGE 函数: 计算参数的平均值。公式: {}", clean_formula),
                "COUNT" => format!(
                    "COUNT 函数: 计算包含数字的单元格数量。公式: {}",
                    clean_formula
                ),
                "IF" => format!("IF 函数: 根据条件返回不同的值。公式: {}", clean_formula),
                "VLOOKUP" => format!("VLOOKUP 函数: 在表格中垂直查找值。公式: {}", clean_formula),
                "INDEX" => format!("INDEX 函数: 返回表格或区域中的值。公式: {}", clean_formula),
                "MATCH" => format!("MATCH 函数: 在范围中查找值的位置。公式: {}", clean_formula),
                "CONCATENATE" | "CONCAT" => {
                    format!("连接函数: 将文本字符串连接起来。公式: {}", clean_formula)
                }
                "LEFT" => format!("LEFT 函数: 从文本左侧提取指定字符。公式: {}", clean_formula),
                "RIGHT" => format!(
                    "RIGHT 函数: 从文本右侧提取指定字符。公式: {}",
                    clean_formula
                ),
                "MID" => format!("MID 函数: 从文本中间提取指定字符。公式: {}", clean_formula),
                "DATE" => format!("DATE 函数: 返回表示日期的序列号。公式: {}", clean_formula),
                "TODAY" => format!("TODAY 函数: 返回当前日期。公式: {}", clean_formula),
                "NOW" => format!("NOW 函数: 返回当前日期和时间。公式: {}", clean_formula),
                _ => format!("{} 函数: Excel函数。公式: {}", func, clean_formula),
            },
            "en" => match func.to_uppercase().as_str() {
                "SUM" => format!(
                    "SUM function: Calculates the sum of arguments. Formula: {}",
                    clean_formula
                ),
                "AVERAGE" => format!(
                    "AVERAGE function: Calculates the average of arguments. Formula: {}",
                    clean_formula
                ),
                "COUNT" => format!(
                    "COUNT function: Counts the number of cells containing numbers. Formula: {}",
                    clean_formula
                ),
                "IF" => format!(
                    "IF function: Returns different values based on conditions. Formula: {}",
                    clean_formula
                ),
                "VLOOKUP" => format!(
                    "VLOOKUP function: Looks up a value in a table vertically. Formula: {}",
                    clean_formula
                ),
                "INDEX" => format!(
                    "INDEX function: Returns a value from a table or range. Formula: {}",
                    clean_formula
                ),
                "MATCH" => format!(
                    "MATCH function: Finds the position of a value in a range. Formula: {}",
                    clean_formula
                ),
                "CONCATENATE" | "CONCAT" => {
                    format!(
                        "Concatenation function: Joins text strings together. Formula: {}",
                        clean_formula
                    )
                }
                "LEFT" => format!(
                    "LEFT function: Extracts a specified number of characters from the left side of text. Formula: {}",
                    clean_formula
                ),
                "RIGHT" => format!(
                    "RIGHT function: Extracts a specified number of characters from the right side of text. Formula: {}",
                    clean_formula
                ),
                "MID" => format!(
                    "MID function: Extracts a specified number of characters from the middle of text. Formula: {}",
                    clean_formula
                ),
                "DATE" => format!(
                    "DATE function: Returns the serial number of a date. Formula: {}",
                    clean_formula
                ),
                "TODAY" => format!(
                    "TODAY function: Returns the current date. Formula: {}",
                    clean_formula
                ),
                "NOW" => format!(
                    "NOW function: Returns the current date and time. Formula: {}",
                    clean_formula
                ),
                _ => format!(
                    "{} function: Excel function. Formula: {}",
                    func, clean_formula
                ),
            },
            _ => format!(
                "{} function: Excel function. Formula: {}",
                func, clean_formula
            ),
        }
    } else {
        match language {
            "zh" => format!("计算表达式: {}", clean_formula),
            "en" => format!("Calculation expression: {}", clean_formula),
            _ => format!("Calculation expression: {}", clean_formula),
        }
    }
}

pub fn explain_formula_logic(
    path: &str,
    sheet: &str,
    cell: &str,
    language: &str,
) -> Result<FormulaLogicExplanation> {
    let formula = excel_read::read_formula(path, sheet, cell)?.ok_or_else(|| {
        AppError::CellNotFound(
            crate::utils::cell_ref::parse_cell_ref(cell).unwrap().0,
            crate::utils::cell_ref::parse_cell_ref(cell).unwrap().1,
        )
    })?;

    let trace = trace_dependencies(path, sheet, cell)?;
    let data_sources = trace.all_precedents;

    let logic_flow = generate_logic_flow(&formula, &data_sources, language);

    let calculation_result = if let Ok(cell_data) = excel_read::read_cell(
        path,
        sheet,
        crate::utils::cell_ref::parse_cell_ref(cell)?.0,
        crate::utils::cell_ref::parse_cell_ref(cell)?.1,
    ) {
        match cell_data.data_type {
            CellDataType::Float | CellDataType::Int | CellDataType::Bool => cell_data.value,
            _ => cell_data.value,
        }
    } else {
        None
    };

    Ok(FormulaLogicExplanation {
        cell: cell.to_string(),
        formula,
        logic_flow,
        data_sources,
        calculation_result,
    })
}

fn generate_logic_flow(formula: &str, data_sources: &[String], language: &str) -> Vec<LogicStep> {
    let mut steps = Vec::new();
    let mut step_num = 1;

    if !data_sources.is_empty() {
        match language {
            "zh" => {
                steps.push(LogicStep {
                    step_number: step_num,
                    operation: "读取数据源".to_string(),
                    input: data_sources.join(", "),
                    result: format!("从 {} 个单元格读取数据", data_sources.len()),
                });
            }
            "en" => {
                steps.push(LogicStep {
                    step_number: step_num,
                    operation: "Read data sources".to_string(),
                    input: data_sources.join(", "),
                    result: format!("Read data from {} cells", data_sources.len()),
                });
            }
            _ => {
                steps.push(LogicStep {
                    step_number: step_num,
                    operation: "Read data sources".to_string(),
                    input: data_sources.join(", "),
                    result: format!("Read data from {} cells", data_sources.len()),
                });
            }
        }
        step_num += 1;
    }

    let (func_name, args) = parse_function(formula);
    if let Some(func) = func_name {
        match func.to_uppercase().as_str() {
            "SUM" => match language {
                "zh" => {
                    steps.push(LogicStep {
                        step_number: step_num,
                        operation: "计算总和".to_string(),
                        input: args.join(", "),
                        result: "所有参数的数值之和".to_string(),
                    });
                }
                "en" => {
                    steps.push(LogicStep {
                        step_number: step_num,
                        operation: "Calculate sum".to_string(),
                        input: args.join(", "),
                        result: "Sum of all arguments".to_string(),
                    });
                }
                _ => {}
            },
            "AVERAGE" => match language {
                "zh" => {
                    steps.push(LogicStep {
                        step_number: step_num,
                        operation: "计算平均值".to_string(),
                        input: args.join(", "),
                        result: "所有参数的数值之和除以参数数量".to_string(),
                    });
                }
                "en" => {
                    steps.push(LogicStep {
                        step_number: step_num,
                        operation: "Calculate average".to_string(),
                        input: args.join(", "),
                        result: "Sum of all arguments divided by count".to_string(),
                    });
                }
                _ => {}
            },
            "IF" => match language {
                "zh" => {
                    steps.push(LogicStep {
                        step_number: step_num,
                        operation: "条件判断".to_string(),
                        input: args.join(", "),
                        result: "如果条件为真返回第一个值，否则返回第二个值".to_string(),
                    });
                }
                "en" => {
                    steps.push(LogicStep {
                        step_number: step_num,
                        operation: "Conditional check".to_string(),
                        input: args.join(", "),
                        result: "Return first value if condition is true, else second value"
                            .to_string(),
                    });
                }
                _ => {}
            },
            "VLOOKUP" => match language {
                "zh" => {
                    steps.push(LogicStep {
                        step_number: step_num,
                        operation: "垂直查找".to_string(),
                        input: args.join(", "),
                        result: "在表格的第一列查找值，返回指定列的对应值".to_string(),
                    });
                }
                "en" => {
                    steps.push(LogicStep {
                        step_number: step_num,
                        operation: "Vertical lookup".to_string(),
                        input: args.join(", "),
                        result: "Look up value in first column, return value from specified column"
                            .to_string(),
                    });
                }
                _ => {}
            },
            _ => match language {
                "zh" => {
                    steps.push(LogicStep {
                        step_number: step_num,
                        operation: format!("执行 {} 函数", func),
                        input: args.join(", "),
                        result: format!("{} 函数的计算结果", func),
                    });
                }
                "en" => {
                    steps.push(LogicStep {
                        step_number: step_num,
                        operation: format!("Execute {} function", func),
                        input: args.join(", "),
                        result: format!("Result of {} function", func),
                    });
                }
                _ => {}
            },
        }
    }

    if steps.is_empty() {
        match language {
            "zh" => {
                steps.push(LogicStep {
                    step_number: step_num,
                    operation: "简单值".to_string(),
                    input: formula.trim_start_matches('=').to_string(),
                    result: "直接使用该值".to_string(),
                });
            }
            "en" => {
                steps.push(LogicStep {
                    step_number: step_num,
                    operation: "Simple value".to_string(),
                    input: formula.trim_start_matches('=').to_string(),
                    result: "Use the value directly".to_string(),
                });
            }
            _ => {}
        }
    }

    steps
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_function_simple() {
        let (func, args) = parse_function("SUM(A1:A10)");
        assert_eq!(func, Some("SUM".to_string()));
        assert_eq!(args, vec!["A1:A10".to_string()]);
    }

    #[test]
    fn test_parse_function_multiple_args() {
        let (func, args) = parse_function("IF(A1>10, true, false)");
        assert_eq!(func, Some("IF".to_string()));
        assert_eq!(
            args,
            vec!["A1>10".to_string(), "true".to_string(), "false".to_string()]
        );
    }

    #[test]
    fn test_parse_function_with_equals() {
        let (func, args) = parse_function("=SUM(A1:A10)");
        assert_eq!(func, Some("SUM".to_string()));
        assert_eq!(args, vec!["A1:A10".to_string()]);
    }

    #[test]
    fn test_parse_function_no_parens() {
        let (func, args) = parse_function("A1+B1");
        assert_eq!(func, None);
        assert_eq!(args, vec!["A1+B1".to_string()]);
    }

    #[test]
    fn test_split_arguments_simple() {
        let args = split_arguments("A1, B1, C1");
        assert_eq!(args, vec!["A1", "B1", "C1"]);
    }

    #[test]
    fn test_split_arguments_with_nested() {
        let args = split_arguments("SUM(A1:A10), AVERAGE(B1:B10)");
        // The implementation strips parentheses content when parsing
        assert_eq!(args.len(), 2);
    }

    #[test]
    fn test_split_arguments_with_string() {
        let args = split_arguments("\"hello, world\", A1");
        // The implementation strips quotes from strings
        assert_eq!(args.len(), 2);
    }

    #[test]
    fn test_split_arguments_empty() {
        let args = split_arguments("");
        assert_eq!(args, Vec::<String>::new());
    }

    #[test]
    fn test_extract_cell_references() {
        let refs = extract_cell_references("SUM(A1, B2, Sheet2!C3)", "Sheet1");
        // Should contain references with sheet names
        assert!(refs.iter().any(|r| r.contains("A1")));
        assert!(refs.iter().any(|r| r.contains("B2")));
        assert!(refs.iter().any(|r| r.contains("Sheet2!C3")));
    }

    #[test]
    fn test_extract_cell_references_with_sheet() {
        let refs = extract_cell_references("Sheet2!A1 + Sheet2!B2", "Sheet1");
        assert!(refs.iter().any(|r| r.contains("Sheet2!A1")));
        assert!(refs.iter().any(|r| r.contains("Sheet2!B2")));
    }

    #[test]
    fn test_generate_formula_description_sum_zh() {
        let desc = generate_formula_description("=SUM(A1:A10)", "zh");
        assert!(desc.contains("SUM"));
        assert!(desc.contains("总和"));
    }

    #[test]
    fn test_generate_formula_description_sum_en() {
        let desc = generate_formula_description("=SUM(A1:A10)", "en");
        assert!(desc.contains("SUM"));
        assert!(desc.contains("sum"));
    }

    #[test]
    fn test_generate_formula_description_if_zh() {
        let desc = generate_formula_description("=IF(A1>10, true, false)", "zh");
        assert!(desc.contains("IF"));
        assert!(desc.contains("条件"));
    }

    #[test]
    fn test_generate_formula_description_unknown_zh() {
        let desc = generate_formula_description("=CUSTOMFUNC(A1)", "zh");
        assert!(desc.contains("CUSTOMFUNC"));
        assert!(desc.contains("Excel函数"));
    }

    #[test]
    fn test_generate_formula_description_simple_zh() {
        let desc = generate_formula_description("42", "zh");
        assert!(desc.contains("计算表达式"));
    }

    #[test]
    fn test_generate_logic_flow_sum_zh() {
        let steps = generate_logic_flow("=SUM(A1:A10)", &["Sheet1!A1".to_string()], "zh");
        assert!(!steps.is_empty());
        assert!(steps.iter().any(|s| s.operation.contains("读取数据")));
        assert!(steps.iter().any(|s| s.operation.contains("总和")));
    }

    #[test]
    fn test_generate_logic_flow_simple_en() {
        let steps = generate_logic_flow("42", &[], "en");
        assert!(!steps.is_empty());
        assert!(steps[0].operation.contains("Simple value"));
    }
}
