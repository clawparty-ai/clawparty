use std::collections::HashMap;
use std::io::Write as IoWrite;
use std::path::PathBuf;

use calamine::{open_workbook_auto, Data, Reader};
use hyper::body::{Bytes, Incoming};
use hyper::{header, Response, StatusCode};
use http_body_util::{BodyExt, combinators::BoxBody};

use crate::proxy::box_body;
use crate::wiki::get_agent_workspace;

fn json_response<T: serde::Serialize>(status: StatusCode, body: &T) -> Response<BoxBody<Bytes, hyper::Error>> {
    let json = serde_json::to_string(body).unwrap_or_default();
    Response::builder()
        .status(status)
        .header(header::CONTENT_TYPE, "application/json")
        .body(box_body(Bytes::from(json)))
        .unwrap()
}

fn error_response(status: StatusCode, message: &str) -> Response<BoxBody<Bytes, hyper::Error>> {
    let body = serde_json::json!({ "error": message });
    json_response(status, &body)
}

fn ok_response<T: serde::Serialize>(body: &T) -> Response<BoxBody<Bytes, hyper::Error>> {
    json_response(StatusCode::OK, body)
}

#[derive(serde::Serialize, Clone)]
struct SheetInfo {
    name: String,
    row_count: usize,
    col_count: usize,
    headers: Vec<String>,
    column_types: Vec<String>,
}

#[derive(serde::Serialize)]
struct DatasetInfo {
    name: String,
    sheets: Vec<SheetInfo>,
    has_formulas: bool,
}

#[derive(serde::Serialize)]
struct ListResponse {
    datasets: Vec<DatasetInfo>,
}

#[derive(serde::Serialize)]
struct UploadResponse {
    status: u16,
    message: String,
    dataset: String,
    sheets: Vec<SheetInfo>,
    has_formulas: bool,
}

struct FormulaCell {
    location: String,
    formula: String,
}

fn sanitize_filename(name: &str) -> String {
    name.replace(['/', '\\', ':', '*', '?', '"', '<', '>', '|'], "_")
}

fn data_to_string(cell: &Data) -> String {
    match cell {
        Data::Empty => String::new(),
        Data::String(s) => s.clone(),
        Data::Float(f) => {
            let s = format!("{}", f);
            if s.ends_with(".0") && s.len() > 2 {
                s[..s.len() - 2].to_string()
            } else {
                s
            }
        }
        Data::Int(i) => i.to_string(),
        Data::DateTime(dt) => dt.to_string(),
        Data::DateTimeIso(s) => s.clone(),
        Data::DurationIso(s) => s.clone(),
        Data::Bool(b) => if *b { "TRUE".to_string() } else { "FALSE".to_string() },
        Data::Error(e) => format!("{:?}", e),
    }
}

fn csv_escape(value: &str) -> String {
    if value.contains(',') || value.contains('"') || value.contains('\n') || value.contains('\r') {
        format!("\"{}\"", value.replace('"', "\"\""))
    } else {
        value.to_string()
    }
}

fn detect_column_type(values: &[String]) -> &'static str {
    let non_empty: Vec<&String> = values.iter().filter(|v| !v.is_empty()).collect();
    if non_empty.is_empty() {
        return "空";
    }
    let num_count = non_empty.iter().filter(|v| v.parse::<f64>().is_ok()).count();
    if num_count as f64 / non_empty.len() as f64 >= 0.9 {
        return "数字";
    }
    let date_count = non_empty.iter().filter(|v| {
        v.len() >= 10 && v.chars().filter(|c| *c == '-').count() >= 2
            && v.chars().nth(4) == Some('-') && v.chars().nth(7) == Some('-')
    }).count();
    if date_count as f64 / non_empty.len() as f64 >= 0.9 {
        return "日期";
    }
    "文本"
}

fn formula_description(formula: &str) -> String {
    let cleaned = formula.trim_start_matches('=');
    let mut desc = cleaned.to_string();

    let replacements: &[(&str, &str)] = &[
        ("SUM(", "求和("),
        ("AVERAGE(", "求平均值("),
        ("COUNT(", "计数("),
        ("COUNTA(", "非空计数("),
        ("COUNTIF(", "条件计数("),
        ("SUMIF(", "条件求和("),
        ("IF(", "条件判断("),
        ("VLOOKUP(", "垂直查找("),
        ("HLOOKUP(", "水平查找("),
        ("MAX(", "最大值("),
        ("MIN(", "最小值("),
        ("ROUND(", "四舍五入("),
        ("AND(", "逻辑与("),
        ("OR(", "逻辑或("),
        ("NOT(", "逻辑非("),
    ];

    for (func, chinese) in replacements {
        if desc.contains(func) {
            desc = desc.replace(func, chinese);
        }
    }
    desc
}

fn parse_workbook(raw_path: &std::path::Path, dataset_dir: &std::path::Path) -> Result<(Vec<SheetInfo>, Vec<(String, Vec<FormulaCell>)>), String> {
    let mut workbook = open_workbook_auto(raw_path)
        .map_err(|e| format!("无法打开 Excel 文件: {}", e))?;

    let sheet_names = workbook.sheet_names();
    let mut sheets = Vec::new();
    let mut all_formulas: Vec<(String, Vec<FormulaCell>)> = Vec::new();

    for sheet_name in &sheet_names {
        let safe_name = sanitize_filename(sheet_name);

        let range = workbook.worksheet_range(sheet_name)
            .map_err(|e| format!("无法读取 Sheet '{}': {}", sheet_name, e))?;

        let total_rows = range.rows().count();
        if total_rows == 0 {
            continue;
        }

        let rows_vec: Vec<Vec<Data>> = range.rows().map(|r| r.to_vec()).collect();
        let total_cols = if rows_vec.is_empty() { 0 } else { rows_vec[0].len() };

        if total_cols == 0 || total_rows == 0 {
            let sheet_info = SheetInfo {
                name: sheet_name.clone(),
                row_count: 0,
                col_count: 0,
                headers: vec![],
                column_types: vec![],
            };
            sheets.push(sheet_info);
            continue;
        }

        let row0_non_empty = rows_vec[0].iter().filter(|c| !matches!(c, Data::Empty)).count();
        let (header_row, description) = if row0_non_empty <= 2 && total_cols > 2 && total_rows > 1 {
            let desc = rows_vec[0].iter()
                .find(|c| !matches!(c, Data::Empty))
                .map(|c| data_to_string(c))
                .unwrap_or_default();
            (1, Some(desc))
        } else {
            (0, None)
        };

        let headers: Vec<String> = if header_row < total_rows {
            rows_vec[header_row].iter().map(|c| data_to_string(c)).collect()
        } else {
            vec![]
        };

        let data_start = header_row + 1;
        let data_rows = if data_start < total_rows { total_rows - data_start } else { 0 };

        let mut column_types = Vec::new();
        for col in 0..total_cols {
            let col_values: Vec<String> = (data_start..total_rows)
                .filter_map(|r| {
                    if col < rows_vec[r].len() {
                        let s = data_to_string(&rows_vec[r][col]);
                        if s.is_empty() { None } else { Some(s) }
                    } else {
                        None
                    }
                })
                .collect();
            column_types.push(detect_column_type(&col_values).to_string());
        }

        let sheet_info = SheetInfo {
            name: sheet_name.clone(),
            row_count: data_rows,
            col_count: total_cols,
            headers,
            column_types: column_types.clone(),
        };

        // Generate CSV
        let sheets_dir = dataset_dir.join("sheets");
        let _ = std::fs::create_dir_all(&sheets_dir);
        let csv_path = sheets_dir.join(format!("{}.csv", safe_name));

        let mut csv_content = String::new();
        let header_str: Vec<String> = rows_vec[header_row].iter().map(|c| csv_escape(&data_to_string(c))).collect();
        csv_content.push_str(&header_str.join(","));
        csv_content.push('\n');

        for r in data_start..total_rows {
            let row_str: Vec<String> = (0..total_cols)
                .map(|c| {
                    let val = if c < rows_vec[r].len() {
                        data_to_string(&rows_vec[r][c])
                    } else {
                        String::new()
                    };
                    csv_escape(&val)
                })
                .collect();
            csv_content.push_str(&row_str.join(","));
            csv_content.push('\n');
        }
        std::fs::write(&csv_path, &csv_content)
            .map_err(|e| format!("无法写入 CSV: {}", e))?;

        // Generate Markdown
        let md_path = sheets_dir.join(format!("{}.md", safe_name));
        let mut md_content = String::new();
        if let Some(ref desc) = description {
            if !desc.is_empty() {
                md_content.push_str(&format!("> {}\n\n", desc));
            }
        }
        md_content.push_str(&format!("# {}\n\n", sheet_name));
        md_content.push_str(&format!("- 数据行数: {}\n", data_rows));
        md_content.push_str(&format!("- 列数: {}\n\n", total_cols));
        md_content.push_str("| ");
        for (i, h) in sheet_info.headers.iter().enumerate() {
            if i > 0 { md_content.push_str(" | "); }
            md_content.push_str(h);
        }
        md_content.push_str(" |\n|");
        for _ in 0..total_cols {
            md_content.push_str("---|");
        }
        md_content.push('\n');
        for r in data_start..total_rows {
            md_content.push('|');
            for c in 0..total_cols {
                let val = if c < rows_vec[r].len() {
                    let s = data_to_string(&rows_vec[r][c]);
                    if s.len() > 200 {
                        let truncated: String = s.chars().take(200).collect();
                        format!("{}...", truncated)
                    } else {
                        s
                    }
                } else {
                    String::new()
                };
                md_content.push(' ');
                md_content.push_str(&val.replace('\n', " ").replace('\r', "").replace('|', "\\|"));
                md_content.push_str(" |");
            }
            md_content.push('\n');
        }
        std::fs::write(&md_path, &md_content)
            .map_err(|e| format!("无法写入 Markdown: {}", e))?;

        // Extract formulas
        if let Ok(formula_range) = workbook.worksheet_formula(sheet_name) {
            let mut sheet_formulas: Vec<FormulaCell> = Vec::new();
            for (row_idx, row) in formula_range.rows().enumerate() {
                for (col_idx, cell) in row.iter().enumerate() {
                    if cell.starts_with('=') {
                        let col_letter = col_idx_to_letter(col_idx);
                        let location = format!("{}{}", col_letter, row_idx + 1);
                        sheet_formulas.push(FormulaCell {
                            location,
                            formula: cell.clone(),
                        });
                    }
                }
            }
            if !sheet_formulas.is_empty() {
                all_formulas.push((sheet_name.clone(), sheet_formulas));
            }
        }

        sheets.push(sheet_info);
    }

    Ok((sheets, all_formulas))
}

fn col_idx_to_letter(idx: usize) -> String {
    let mut n = idx;
    let mut result = String::new();
    loop {
        let rem = (n % 26) as u8;
        result.insert(0, (b'A' + rem) as char);
        n = n / 26;
        if n == 0 { break; }
        n -= 1;
    }
    result
}

fn generate_overview_md(sheets: &[SheetInfo], has_formulas: bool, output_path: &std::path::Path) {
    let mut md = String::new();
    md.push_str("# Excel 数据总览\n\n");
    md.push_str(&format!("- Sheet 数量: {}\n", sheets.len()));
    let total_rows: usize = sheets.iter().map(|s| s.row_count).sum();
    md.push_str(&format!("- 总数据行数: {}\n", total_rows));
    md.push_str(&format!("- 包含公式: {}\n\n", if has_formulas { "是" } else { "否" }));

    md.push_str("## Sheet 列表\n\n");
    md.push_str("| Sheet | 行数 | 列数 | 列类型 |\n");
    md.push_str("|-------|------|------|--------|\n");
    for s in sheets {
        let types_summary: Vec<String> = s.column_types.iter()
            .enumerate()
            .map(|(i, t)| {
                let header = s.headers.get(i).map(|h| h.as_str()).unwrap_or("?");
                format!("{}({})", header, t)
            })
            .collect();
        md.push_str(&format!("| {} | {} | {} | {} |\n",
            s.name, s.row_count, s.col_count, types_summary.join(", ")));
    }

    let _ = std::fs::write(output_path, md);
}

fn generate_formulas_md(all_formulas: &[(String, Vec<FormulaCell>)], output_path: &std::path::Path) {
    let mut md = String::new();
    md.push_str("# 公式清单\n\n");

    if all_formulas.is_empty() {
        md.push_str("本工作簿未包含公式。\n");
    } else {
        for (sheet_name, formulas) in all_formulas {
            md.push_str(&format!("## {}\n\n", sheet_name));
            md.push_str("| 单元格 | 公式 | 说明 |\n");
            md.push_str("|--------|------|------|\n");
            for f in formulas {
                let desc = formula_description(&f.formula);
                md.push_str(&format!("| {} | `{}` | {} |\n", f.location, f.formula, desc));
            }
            md.push('\n');
        }
    }

    let _ = std::fs::write(output_path, md);
}

// ── API Handlers ──

async fn handle_upload(data_dir: &str, agent_name: &str, filename: &str, body: Bytes) -> Response<BoxBody<Bytes, hyper::Error>> {
    if filename.is_empty() {
        return error_response(StatusCode::BAD_REQUEST, "name is required");
    }
    if filename.contains("..") || filename.contains('/') || filename.contains('\\') {
        return error_response(StatusCode::FORBIDDEN, "Forbidden filename");
    }

    let basename = filename
        .rsplit_once('.')
        .map(|(name, _)| name.to_string())
        .unwrap_or_else(|| filename.to_string());
    let basename = sanitize_filename(&basename);

    let workspace = match get_agent_workspace(data_dir, agent_name) {
        Ok(w) => w,
        Err(_) => return error_response(StatusCode::NOT_FOUND, "Agent not found"),
    };

    let e2a_dir = workspace.join("e2a");
    let _ = tokio::fs::create_dir_all(&e2a_dir).await;
    let dataset_dir = e2a_dir.join(&basename);

    // Clean up existing dataset if re-uploading
    if dataset_dir.exists() {
        let _ = tokio::fs::remove_dir_all(&dataset_dir).await;
    }
    let _ = tokio::fs::create_dir_all(&dataset_dir).await;

    let raw_path = dataset_dir.join("raw.xlsx");
    if let Err(e) = tokio::fs::write(&raw_path, &body).await {
        return error_response(StatusCode::INTERNAL_SERVER_ERROR, &format!("Failed to save file: {}", e));
    }

    let (sheets, all_formulas) = match parse_workbook(&raw_path, &dataset_dir) {
        Ok(result) => result,
        Err(e) => {
            let _ = tokio::fs::remove_dir_all(&dataset_dir).await;
            return error_response(StatusCode::BAD_REQUEST, &e);
        }
    };

    let has_formulas = !all_formulas.is_empty();
    let formulas_path = dataset_dir.join("formulas.md");
    generate_formulas_md(&all_formulas, &formulas_path);

    let overview_path = dataset_dir.join("overview.md");
    generate_overview_md(&sheets, has_formulas, &overview_path);

    ok_response(&UploadResponse {
        status: 200,
        message: "Excel parsed successfully".to_string(),
        dataset: basename,
        sheets,
        has_formulas,
    })
}

async fn handle_list(data_dir: &str, agent_name: &str) -> Response<BoxBody<Bytes, hyper::Error>> {
    let workspace = match get_agent_workspace(data_dir, agent_name) {
        Ok(w) => w,
        Err(_) => return error_response(StatusCode::NOT_FOUND, "Agent not found"),
    };

    let e2a_dir = workspace.join("e2a");
    if !e2a_dir.exists() {
        return ok_response(&ListResponse { datasets: vec![] });
    }

    let mut entries = match tokio::fs::read_dir(&e2a_dir).await {
        Ok(e) => e,
        Err(_) => return ok_response(&ListResponse { datasets: vec![] }),
    };

    let mut datasets = Vec::new();
    while let Ok(Some(entry)) = entries.next_entry().await {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let name = entry.file_name().to_string_lossy().to_string();
        if name.starts_with('.') {
            continue;
        }

        let overview_path = path.join("overview.md");
        let mut sheets = Vec::new();
        let mut has_formulas = false;

        if overview_path.exists() {
            if let Ok(content) = tokio::fs::read_to_string(&overview_path).await {
                for line in content.lines() {
                    if line.starts_with("| ") && !line.starts_with("| Sheet") && !line.starts_with("|--") && !line.starts_with("| -") {
                        let parts: Vec<&str> = line.split('|').map(|s| s.trim()).collect();
                        if parts.len() >= 5 {
                            let sheet_name = parts.get(1).unwrap_or(&"").to_string();
                            if sheet_name == name || sheet_name.is_empty() { continue; }
                            let row_count: usize = parts.get(2).unwrap_or(&"0").parse().unwrap_or(0);
                            let col_count: usize = parts.get(3).unwrap_or(&"0").parse().unwrap_or(0);
                            sheets.push(SheetInfo {
                                name: sheet_name,
                                row_count,
                                col_count,
                                headers: vec![],
                                column_types: vec![],
                            });
                        }
                    }
                    if line.contains("公式: 是") {
                        has_formulas = true;
                    }
                }
            }
        }

        // Also check sheets directory
        let sheets_dir = path.join("sheets");
        if sheets_dir.exists() && sheets.is_empty() {
            if let Ok(mut dir_entries) = tokio::fs::read_dir(&sheets_dir).await {
                while let Ok(Some(de)) = dir_entries.next_entry().await {
                    let fname = de.file_name().to_string_lossy().to_string();
                    if fname.ends_with(".csv") && !fname.starts_with('.') {
                        let sheet_name = fname.trim_end_matches(".csv").to_string();
                        let row_count = 0usize;
                        let col_count = 0usize;
                        sheets.push(SheetInfo {
                            name: sheet_name,
                            row_count,
                            col_count,
                            headers: vec![],
                            column_types: vec![],
                        });
                    }
                }
            }
        }

        let formulas_path = path.join("formulas.md");
        if formulas_path.exists() {
            if let Ok(content) = tokio::fs::read_to_string(&formulas_path).await {
                if !content.contains("未包含公式") && content.contains("| 单元格 |") {
                    has_formulas = true;
                }
            }
        }

        datasets.push(DatasetInfo {
            name,
            sheets,
            has_formulas,
        });
    }

    ok_response(&ListResponse { datasets })
}

async fn handle_file(data_dir: &str, agent_name: &str, dataset: &str, filename: &str) -> Response<BoxBody<Bytes, hyper::Error>> {
    if dataset.contains("..") || dataset.contains('/') || dataset.contains('\\') {
        return error_response(StatusCode::FORBIDDEN, "Forbidden");
    }
    if filename.contains("..") || filename.contains('/') || filename.contains('\\') {
        return error_response(StatusCode::FORBIDDEN, "Forbidden");
    }

    let workspace = match get_agent_workspace(data_dir, agent_name) {
        Ok(w) => w,
        Err(_) => return error_response(StatusCode::NOT_FOUND, "Agent not found"),
    };

    let file_path = workspace.join("e2a").join(dataset).join(filename);
    if !file_path.starts_with(&workspace.join("e2a")) {
        return error_response(StatusCode::FORBIDDEN, "Forbidden path");
    }

    let data = match tokio::fs::read(&file_path).await {
        Ok(d) => d,
        Err(_) => {
            let alt_path = workspace.join("e2a").join(dataset).join("sheets").join(filename);
            match tokio::fs::read(&alt_path).await {
                Ok(d) => d,
                Err(_) => return error_response(StatusCode::NOT_FOUND, "File not found"),
            }
        }
    };

    let mime = if filename.ends_with(".csv") {
        "text/csv; charset=utf-8"
    } else if filename.ends_with(".md") {
        "text/markdown; charset=utf-8"
    } else {
        "application/octet-stream"
    };

    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, mime)
        .body(box_body(Bytes::from(data)))
        .unwrap()
}

async fn handle_delete(data_dir: &str, agent_name: &str, dataset: &str) -> Response<BoxBody<Bytes, hyper::Error>> {
    if dataset.contains("..") || dataset.contains('/') || dataset.contains('\\') {
        return error_response(StatusCode::FORBIDDEN, "Forbidden");
    }

    let workspace = match get_agent_workspace(data_dir, agent_name) {
        Ok(w) => w,
        Err(_) => return error_response(StatusCode::NOT_FOUND, "Agent not found"),
    };

    let dataset_dir = workspace.join("e2a").join(dataset);
    if !dataset_dir.starts_with(&workspace.join("e2a")) {
        return error_response(StatusCode::FORBIDDEN, "Forbidden path");
    }

    match tokio::fs::remove_dir_all(&dataset_dir).await {
        Ok(_) => ok_response(&serde_json::json!({ "message": "Deleted" })),
        Err(e) => error_response(StatusCode::INTERNAL_SERVER_ERROR, &format!("Failed to delete: {}", e)),
    }
}

/// Route dispatcher for /api/e2a/* requests.
pub async fn route(
    data_dir: &str,
    path: &str,
    method: &hyper::Method,
    req: hyper::Request<Incoming>,
) -> Option<Response<BoxBody<Bytes, hyper::Error>>> {
    let rest = path.strip_prefix("/api/e2a/")?;

    let segments: Vec<&str> = rest.split('/').collect();
    if segments.is_empty() || segments[0].is_empty() {
        return None;
    }

    let agent_encoded = segments[0];
    let agent = urlencoding::decode(agent_encoded).unwrap_or_else(|_| agent_encoded.into()).to_string();

    let query = req.uri().query().unwrap_or("");

    match segments.len() {
        1 => {
            // /api/e2a/{agent}
            if method != hyper::Method::DELETE {
                return None;
            }
            // DELETE with dataset from query
            let dataset = url::form_urlencoded::parse(query.as_bytes())
                .find(|(k, _)| k == "dataset")
                .map(|(_, v)| v.to_string())
                .unwrap_or_default();
            if dataset.is_empty() {
                return Some(error_response(StatusCode::BAD_REQUEST, "dataset query param required"));
            }
            Some(handle_delete(data_dir, &agent, &dataset).await)
        }
        _ => {
            let action = segments[1];
            match action {
                "upload" if method == hyper::Method::POST => {
                    let filename = url::form_urlencoded::parse(query.as_bytes())
                        .find(|(k, _)| k == "name")
                        .map(|(_, v)| v.to_string())
                        .unwrap_or_default();
                    let body_bytes = match req.collect().await {
                        Ok(body) => body.to_bytes(),
                        Err(_) => {
                            return Some(error_response(StatusCode::BAD_REQUEST, "Failed to read body"));
                        }
                    };
                    Some(handle_upload(data_dir, &agent, &filename, body_bytes).await)
                }
                "list" if method == hyper::Method::GET => {
                    Some(handle_list(data_dir, &agent).await)
                }
                "file" if method == hyper::Method::GET && segments.len() >= 4 => {
                    let dataset = urlencoding::decode(segments[2]).unwrap_or_else(|_| segments[2].into()).to_string();
                    let fname_enc = segments[3..].join("/");
                    let filename = urlencoding::decode(&fname_enc).map(|c| c.into_owned()).unwrap_or(fname_enc);
                    Some(handle_file(data_dir, &agent, &dataset, &filename).await)
                }
                _ => {
                    // Check if action is a dataset name (for DELETE)
                    if method == hyper::Method::DELETE {
                        Some(handle_delete(data_dir, &agent, action).await)
                    } else {
                        None
                    }
                }
            }
        }
    }
}
