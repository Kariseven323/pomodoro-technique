//! 导出相关命令：将历史记录导出为 CSV/JSON。

use serde::Serialize;

use crate::app_data::{DateRange, HistoryDay, HistoryRecord, Phase};
use crate::errors::{AppError, AppResult};

use super::history::get_history_impl;
use super::state_like::CommandState;
use super::types::{ExportField, ExportFormat, ExportRequest};
use super::validation::validate_date_range;

/// 将导出请求写入指定路径（用于测试与复用：不依赖系统文件对话框）。
pub(crate) fn export_history_to_path<S: CommandState>(
    state: &S,
    request: &ExportRequest,
    path: &std::path::Path,
) -> AppResult<()> {
    validate_date_range(&request.range)?;
    let fields = normalize_export_fields(request.fields.clone());

    let days = get_history_impl(state, &request.range)?;
    let export_rows = flatten_days_to_rows(&days);

    match request.format {
        ExportFormat::Csv => export_csv(path, &fields, &export_rows)?,
        ExportFormat::Json => export_json(path, &request.range, &export_rows)?,
    }
    Ok(())
}

/// 将导出字段列表规范化：当为空时回退到默认字段集合（PRD v2）。
fn normalize_export_fields(mut fields: Vec<ExportField>) -> Vec<ExportField> {
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
    fields
}

/// 生成导出默认文件名（范围 + 格式扩展名）。
pub(crate) fn default_export_file_name(range: &DateRange, format: ExportFormat) -> String {
    let ext = match format {
        ExportFormat::Csv => "csv",
        ExportFormat::Json => "json",
    };
    format!("pomodoro-history-{}-{}.{}", range.from, range.to, ext)
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

#[cfg(test)]
mod tests {
    use super::*;

    use crate::app_data::AppData;
    use crate::commands::state_like::TestState;

    /// `derive_end_time_hhmm`：应支持合法时间与跨日回绕。
    #[test]
    fn derive_end_time_handles_wrap() {
        assert_eq!(derive_end_time_hhmm("09:00", 25).as_deref(), Some("09:25"));
        assert_eq!(derive_end_time_hhmm("23:50", 20).as_deref(), Some("00:10"));
    }

    /// `derive_end_time_hhmm`：非法输入应返回 None。
    #[test]
    fn derive_end_time_rejects_invalid_input() {
        assert_eq!(derive_end_time_hhmm("bad", 25), None);
        assert_eq!(derive_end_time_hhmm("24:00", 25), None);
        assert_eq!(derive_end_time_hhmm("23:99", 25), None);
    }

    /// `flatten_days_to_rows`：应按记录数拉平为导出行。
    #[test]
    fn flatten_days_to_rows_flattens() {
        let days = vec![HistoryDay {
            date: "2025-01-01".to_string(),
            records: vec![
                HistoryRecord {
                    tag: "A".to_string(),
                    start_time: "09:00".to_string(),
                    end_time: None,
                    duration: 25,
                    phase: Phase::Work,
                    remark: String::new(),
                },
                HistoryRecord {
                    tag: "B".to_string(),
                    start_time: "10:00".to_string(),
                    end_time: None,
                    duration: 5,
                    phase: Phase::ShortBreak,
                    remark: String::new(),
                },
            ],
        }];

        let rows = flatten_days_to_rows(&days);
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].date, "2025-01-01");
        assert_eq!(rows[0].record.tag, "A");
        assert_eq!(rows[1].record.tag, "B");
    }

    /// `export_csv`：应按字段顺序写入表头与行，并在缺失 end_time 时推导。
    #[test]
    fn export_csv_writes_expected_content() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("out.csv");

        let days = vec![HistoryDay {
            date: "2025-01-01".to_string(),
            records: vec![HistoryRecord {
                tag: "A".to_string(),
                start_time: "09:00".to_string(),
                end_time: None,
                duration: 25,
                phase: Phase::Work,
                remark: "hi".to_string(),
            }],
        }];
        let rows = flatten_days_to_rows(&days);

        export_csv(
            &path,
            &[
                ExportField::Date,
                ExportField::StartTime,
                ExportField::EndTime,
                ExportField::Duration,
                ExportField::Tag,
                ExportField::Phase,
                ExportField::Remark,
            ],
            &rows,
        )
        .unwrap();

        let content = std::fs::read_to_string(&path).unwrap();
        let lines: Vec<&str> = content.lines().collect();
        assert_eq!(lines[0], "date,start_time,end_time,duration,tag,phase,remark");
        assert_eq!(lines[1], "2025-01-01,09:00,09:25,25,A,work,hi");
    }

    /// `export_json`：应写入可解析 JSON，且包含 range 与 records。
    #[test]
    fn export_json_writes_parseable_json() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("out.json");

        let range = DateRange {
            from: "2025-01-01".to_string(),
            to: "2025-01-07".to_string(),
        };
        let days = vec![HistoryDay {
            date: "2025-01-01".to_string(),
            records: vec![HistoryRecord {
                tag: "A".to_string(),
                start_time: "09:00".to_string(),
                end_time: None,
                duration: 25,
                phase: Phase::Work,
                remark: String::new(),
            }],
        }];
        let rows = flatten_days_to_rows(&days);

        export_json(&path, &range, &rows).unwrap();
        let content = std::fs::read_to_string(&path).unwrap();
        let v: serde_json::Value = serde_json::from_str(&content).unwrap();
        assert_eq!(v["range"]["from"], "2025-01-01");
        assert_eq!(v["range"]["to"], "2025-01-07");
        assert_eq!(v["records"].as_array().unwrap().len(), 1);
        assert_eq!(v["records"][0]["date"], "2025-01-01");
        assert_eq!(v["records"][0]["endTime"], "09:25");
    }

    /// `normalize_export_fields`：当入参为空时应回退到默认字段集合。
    #[test]
    fn normalize_export_fields_falls_back_to_default() {
        let out = normalize_export_fields(Vec::new());
        assert_eq!(out.len(), 6);
        assert!(matches!(out[0], ExportField::Date));
        assert!(matches!(out[1], ExportField::StartTime));
        assert!(matches!(out[2], ExportField::EndTime));
        assert!(matches!(out[3], ExportField::Duration));
        assert!(matches!(out[4], ExportField::Tag));
        assert!(matches!(out[5], ExportField::Phase));
    }

    /// `default_export_file_name`：应根据 range 与 format 生成可读文件名。
    #[test]
    fn default_export_file_name_uses_range_and_ext() {
        let name = default_export_file_name(
            &DateRange {
                from: "2025-01-01".to_string(),
                to: "2025-01-07".to_string(),
            },
            ExportFormat::Csv,
        );
        assert_eq!(name, "pomodoro-history-2025-01-01-2025-01-07.csv");
    }

    /// `default_export_file_name`：JSON 格式应使用 .json 扩展名。
    #[test]
    fn default_export_file_name_uses_json_ext() {
        let name = default_export_file_name(
            &DateRange {
                from: "2025-01-01".to_string(),
                to: "2025-01-07".to_string(),
            },
            ExportFormat::Json,
        );
        assert_eq!(name, "pomodoro-history-2025-01-01-2025-01-07.json");
    }

    /// `export_history_to_path`：应按请求写入 CSV，并在 fields 为空时应用默认字段集。
    #[test]
    fn export_history_to_path_writes_csv_with_default_fields() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("history.csv");

        let mut data = AppData::default();
        data.history_dev = vec![HistoryDay {
            date: "2025-01-01".to_string(),
            records: vec![HistoryRecord {
                tag: "A".to_string(),
                start_time: "09:00".to_string(),
                end_time: None,
                duration: 25,
                phase: Phase::Work,
                remark: String::new(),
            }],
        }];
        let state = TestState::new(data);

        export_history_to_path(
            &state,
            &ExportRequest {
                format: ExportFormat::Csv,
                range: DateRange {
                    from: "2025-01-01".to_string(),
                    to: "2025-01-01".to_string(),
                },
                fields: Vec::new(),
            },
            &path,
        )
        .unwrap();

        let content = std::fs::read_to_string(&path).unwrap();
        let header = content.lines().next().unwrap();
        assert_eq!(header, "date,start_time,end_time,duration,tag,phase");
        assert!(content.contains("2025-01-01,09:00,09:25,25,A,work"));
    }

    /// `export_history_to_path`：应按请求写入 JSON（结构正确即可）。
    #[test]
    fn export_history_to_path_writes_json() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("history.json");

        let mut data = AppData::default();
        data.history_dev = vec![HistoryDay {
            date: "2025-01-01".to_string(),
            records: vec![HistoryRecord {
                tag: "A".to_string(),
                start_time: "09:00".to_string(),
                end_time: None,
                duration: 25,
                phase: Phase::Work,
                remark: String::new(),
            }],
        }];
        let state = TestState::new(data);

        export_history_to_path(
            &state,
            &ExportRequest {
                format: ExportFormat::Json,
                range: DateRange {
                    from: "2025-01-01".to_string(),
                    to: "2025-01-01".to_string(),
                },
                fields: Vec::new(),
            },
            &path,
        )
        .unwrap();

        let content = std::fs::read_to_string(&path).unwrap();
        let v: serde_json::Value = serde_json::from_str(&content).unwrap();
        assert_eq!(v["range"]["from"], "2025-01-01");
        assert_eq!(v["records"].as_array().unwrap().len(), 1);
    }

    /// `export_history_to_path`：CSV 导出应正确写出 work/shortBreak/longBreak 的 phase 字符串。
    #[test]
    fn export_csv_includes_all_phase_strings() {
        let mut data = crate::app_data::AppData::default();
        data.history = vec![HistoryDay {
            date: "2025-01-01".to_string(),
            records: vec![
                HistoryRecord {
                    tag: "A".to_string(),
                    start_time: "09:00".to_string(),
                    end_time: Some("09:25".to_string()),
                    duration: 25,
                    phase: Phase::Work,
                    remark: String::new(),
                },
                HistoryRecord {
                    tag: "B".to_string(),
                    start_time: "09:30".to_string(),
                    end_time: Some("09:35".to_string()),
                    duration: 5,
                    phase: Phase::ShortBreak,
                    remark: String::new(),
                },
                HistoryRecord {
                    tag: "C".to_string(),
                    start_time: "10:00".to_string(),
                    end_time: Some("10:15".to_string()),
                    duration: 15,
                    phase: Phase::LongBreak,
                    remark: String::new(),
                },
            ],
        }];
        let state = crate::commands::state_like::TestState::new(data);

        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("out.csv");
        let request = ExportRequest {
            range: DateRange {
                from: "2025-01-01".to_string(),
                to: "2025-01-01".to_string(),
            },
            format: ExportFormat::Csv,
            fields: Vec::new(),
        };

        export_history_to_path(&state, &request, &path).unwrap();
        let content = std::fs::read_to_string(&path).unwrap();
        assert!(content.contains("work"));
        assert!(content.contains("shortBreak"));
        assert!(content.contains("longBreak"));
    }

    /// `export_history_to_path`：JSON 导出应正确写出 work/shortBreak/longBreak 的 phase 字符串。
    #[test]
    fn export_json_includes_all_phase_strings() {
        let mut data = crate::app_data::AppData::default();
        data.history = vec![HistoryDay {
            date: "2025-01-01".to_string(),
            records: vec![
                HistoryRecord {
                    tag: "A".to_string(),
                    start_time: "09:00".to_string(),
                    end_time: Some("09:25".to_string()),
                    duration: 25,
                    phase: Phase::Work,
                    remark: String::new(),
                },
                HistoryRecord {
                    tag: "B".to_string(),
                    start_time: "09:30".to_string(),
                    end_time: Some("09:35".to_string()),
                    duration: 5,
                    phase: Phase::ShortBreak,
                    remark: String::new(),
                },
                HistoryRecord {
                    tag: "C".to_string(),
                    start_time: "10:00".to_string(),
                    end_time: Some("10:15".to_string()),
                    duration: 15,
                    phase: Phase::LongBreak,
                    remark: String::new(),
                },
            ],
        }];
        let state = crate::commands::state_like::TestState::new(data);

        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("out.json");
        let request = ExportRequest {
            range: DateRange {
                from: "2025-01-01".to_string(),
                to: "2025-01-01".to_string(),
            },
            format: ExportFormat::Json,
            fields: Vec::new(),
        };

        export_history_to_path(&state, &request, &path).unwrap();
        let content = std::fs::read_to_string(&path).unwrap();
        assert!(content.contains("\"work\""));
        assert!(content.contains("\"shortBreak\""));
        assert!(content.contains("\"longBreak\""));
    }
}
