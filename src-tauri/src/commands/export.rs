//! 导出相关命令：将历史记录导出为 CSV/JSON。

use serde::Serialize;
use tauri_plugin_dialog::DialogExt as _;

use crate::app_data::{DateRange, HistoryDay, HistoryRecord, Phase};
use crate::errors::{AppError, AppResult};
use crate::state::AppState;

use super::common::to_ipc_result;
use super::history::get_history_impl;
use super::types::{ExportField, ExportFormat, ExportRequest};
use super::validation::validate_date_range;

/// 导出历史记录：弹出保存对话框并写入 CSV/JSON，返回保存的文件路径。
#[tauri::command]
pub async fn export_history(
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
    request: ExportRequest,
) -> Result<String, String> {
    to_ipc_result(export_history_impl(&app, &state, request))
}

/// 导出历史的内部实现：校验范围、弹出保存对话框、写入文件。
fn export_history_impl(
    app: &tauri::AppHandle,
    state: &AppState,
    request: ExportRequest,
) -> AppResult<String> {
    validate_date_range(&request.range)?;

    let mut fields = request.fields;
    if fields.is_empty() {
        fields = vec![
            ExportField::Date,
            ExportField::StartTime,
            ExportField::EndTime,
            ExportField::Duration,
            ExportField::Tag,
            ExportField::Phase,
        ];
    }

    let ext = match request.format {
        ExportFormat::Csv => "csv",
        ExportFormat::Json => "json",
    };

    let default_name = format!(
        "pomodoro-history-{}-{}.{}",
        request.range.from, request.range.to, ext
    );

    // 后端弹出系统保存对话框（PRD v2）。
    let Some(path) = app
        .dialog()
        .file()
        .set_file_name(&default_name)
        .blocking_save_file()
    else {
        return Err(AppError::Validation("已取消导出".to_string()));
    };

    let path = path
        .into_path()
        .map_err(|_| AppError::Invariant("导出路径解析失败".to_string()))?;

    let days = get_history_impl(state, &request.range)?;
    let export_rows = flatten_days_to_rows(&days);

    match request.format {
        ExportFormat::Csv => export_csv(&path, &fields, &export_rows)?,
        ExportFormat::Json => export_json(&path, &request.range, &export_rows)?,
    }

    tracing::info!(target: "storage", "导出历史成功：path={}", path.to_string_lossy());
    Ok(path.to_string_lossy().to_string())
}

/// 将按日分组的历史拉平成导出行（每条记录一行）。
fn flatten_days_to_rows(days: &[HistoryDay]) -> Vec<ExportRow> {
    let mut out = Vec::new();
    for day in days {
        for r in &day.records {
            out.push(ExportRow {
                date: day.date.clone(),
                record: r.clone(),
            });
        }
    }
    out
}

/// 单条导出行：`date + record`。
#[derive(Debug, Clone)]
struct ExportRow {
    date: String,
    record: HistoryRecord,
}

/// 将 `startTime + duration` 推导出 `endTime`（用于旧数据缺失 `end_time` 的兼容）。
fn derive_end_time_hhmm(start_time: &str, duration_minutes: u32) -> Option<String> {
    let parts: Vec<&str> = start_time.split(':').collect();
    if parts.len() != 2 {
        return None;
    }
    let h: i32 = parts[0].parse().ok()?;
    let m: i32 = parts[1].parse().ok()?;
    if !(0..=23).contains(&h) || !(0..=59).contains(&m) {
        return None;
    }
    let total = h * 60 + m + duration_minutes as i32;
    let hh = ((total / 60) % 24 + 24) % 24;
    let mm = ((total % 60) + 60) % 60;
    Some(format!("{:02}:{:02}", hh, mm))
}

/// 导出 CSV 文件（字段可配置）。
fn export_csv(path: &std::path::Path, fields: &[ExportField], rows: &[ExportRow]) -> AppResult<()> {
    let file = std::fs::File::create(path)
        .map_err(|e| AppError::Invariant(format!("创建导出文件失败：{e}")))?;
    let mut wtr = csv::Writer::from_writer(file);

    let header: Vec<&str> = fields
        .iter()
        .map(|f| match f {
            ExportField::Date => "date",
            ExportField::StartTime => "start_time",
            ExportField::EndTime => "end_time",
            ExportField::Duration => "duration",
            ExportField::Tag => "tag",
            ExportField::Phase => "phase",
            ExportField::Remark => "remark",
        })
        .collect();
    wtr.write_record(&header)
        .map_err(|e| AppError::Invariant(format!("写入 CSV 头失败：{e}")))?;

    for row in rows {
        let mut record: Vec<String> = Vec::new();
        for f in fields {
            let v = match f {
                ExportField::Date => row.date.clone(),
                ExportField::StartTime => row.record.start_time.clone(),
                ExportField::EndTime => row
                    .record
                    .end_time
                    .clone()
                    .or_else(|| derive_end_time_hhmm(&row.record.start_time, row.record.duration))
                    .unwrap_or_default(),
                ExportField::Duration => row.record.duration.to_string(),
                ExportField::Tag => row.record.tag.clone(),
                ExportField::Phase => match row.record.phase {
                    Phase::Work => "work".to_string(),
                    Phase::ShortBreak => "shortBreak".to_string(),
                    Phase::LongBreak => "longBreak".to_string(),
                },
                ExportField::Remark => row.record.remark.clone(),
            };
            record.push(v);
        }
        wtr.write_record(&record)
            .map_err(|e| AppError::Invariant(format!("写入 CSV 行失败：{e}")))?;
    }
    wtr.flush()
        .map_err(|e| AppError::Invariant(format!("写入 CSV 失败：{e}")))?;
    Ok(())
}

/// JSON 导出文件顶层结构。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct JsonExport {
    export_date: String,
    range: DateRange,
    records: Vec<JsonExportRecord>,
}

/// JSON 导出单条记录结构。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct JsonExportRecord {
    date: String,
    start_time: String,
    end_time: String,
    duration: u32,
    tag: String,
    phase: String,
    remark: String,
}

/// 导出 JSON 文件（字段固定为 PRD v2 示例的 superset）。
fn export_json(path: &std::path::Path, range: &DateRange, rows: &[ExportRow]) -> AppResult<()> {
    let today = chrono::Local::now().format("%Y-%m-%d").to_string();
    let mut records: Vec<JsonExportRecord> = Vec::new();
    for row in rows {
        let end_time = row
            .record
            .end_time
            .clone()
            .or_else(|| derive_end_time_hhmm(&row.record.start_time, row.record.duration))
            .unwrap_or_default();
        let phase = match row.record.phase {
            Phase::Work => "work",
            Phase::ShortBreak => "shortBreak",
            Phase::LongBreak => "longBreak",
        }
        .to_string();
        records.push(JsonExportRecord {
            date: row.date.clone(),
            start_time: row.record.start_time.clone(),
            end_time,
            duration: row.record.duration,
            tag: row.record.tag.clone(),
            phase,
            remark: row.record.remark.clone(),
        });
    }

    let out = JsonExport {
        export_date: today,
        range: range.clone(),
        records,
    };

    let json = serde_json::to_string_pretty(&out)?;
    std::fs::write(path, json).map_err(|e| AppError::Invariant(format!("写入 JSON 失败：{e}")))?;
    Ok(())
}
