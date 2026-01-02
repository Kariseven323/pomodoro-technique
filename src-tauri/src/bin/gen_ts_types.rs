//! 生成前端共享 TypeScript 类型定义文件（基于 `ts-rs`）。

use std::path::PathBuf;

use ts_rs::TS as _;

use tauri_app_lib::typegen::{
    AppData, AppSnapshot, BlacklistItem, BlacklistTemplate, DateRange, ExportField, ExportFormat,
    ExportRequest, FocusAnalysis, GoalProgress, HistoryDay, HistoryRecord, KillSummary, Phase,
    KillItem, ProcessInfo, Settings, StorePaths, TagCount, TagEfficiency, TimerSnapshot, TodayStats,
    WeekStats, WorkCompletedEvent,
};

/// 解析输出路径参数：支持 `--out <path>`，否则写入默认位置。
fn resolve_out_path(args: &[String]) -> PathBuf {
    if let Some(pos) = args.iter().position(|a| a == "--out") {
        if let Some(p) = args.get(pos + 1) {
            return PathBuf::from(p);
        }
    }

    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest_dir
        .parent()
        .unwrap_or(&manifest_dir)
        .join("src/lib/shared/types.generated.ts")
}

/// 生成 `types.generated.ts` 的完整内容（包含所有需要的共享类型声明）。
fn build_types_file() -> String {
    let mut out = String::new();
    out.push_str("/**\n");
    out.push_str(" * 本文件由 `cargo run --bin gen_ts_types` 自动生成，请勿手动编辑。\n");
    out.push_str(" *\n");
    out.push_str(" * 生成来源：Rust 端 `ts-rs` derive（序列化字段名与前端保持 camelCase 一致）。\n");
    out.push_str(" */\n\n");

    // PRD 数据结构（AppData + TimerSnapshot + 进程/分析/导出等命令类型）。
    out.push_str(&exported_decl(&Phase::decl()));
    out.push('\n');
    out.push_str(&exported_decl(&Settings::decl()));
    out.push('\n');
    out.push_str(&exported_decl(&BlacklistItem::decl()));
    out.push('\n');
    out.push_str(&exported_decl(&BlacklistTemplate::decl()));
    out.push('\n');
    out.push_str(&exported_decl(&HistoryRecord::decl()));
    out.push('\n');
    out.push_str(&exported_decl(&HistoryDay::decl()));
    out.push('\n');
    out.push_str(&exported_decl(&AppData::decl()));
    out.push('\n');

    out.push_str(&exported_decl(&TagCount::decl()));
    out.push('\n');
    out.push_str(&exported_decl(&TodayStats::decl()));
    out.push('\n');
    out.push_str(&exported_decl(&WeekStats::decl()));
    out.push('\n');
    out.push_str(&exported_decl(&GoalProgress::decl()));
    out.push('\n');
    out.push_str(&exported_decl(&TimerSnapshot::decl()));
    out.push('\n');
    out.push_str(&exported_decl(&WorkCompletedEvent::decl()));
    out.push('\n');

    out.push_str(&exported_decl(&AppSnapshot::decl()));
    out.push('\n');
    out.push_str(&exported_decl(&StorePaths::decl()));
    out.push('\n');

    out.push_str(&exported_decl(&ProcessInfo::decl()));
    out.push('\n');
    out.push_str(&exported_decl(&KillItem::decl()));
    out.push('\n');
    out.push_str(&exported_decl(&KillSummary::decl()));
    out.push('\n');

    out.push_str(&exported_decl(&DateRange::decl()));
    out.push('\n');

    out.push_str(&exported_decl(&TagEfficiency::decl()));
    out.push('\n');
    out.push_str(&exported_decl(&FocusAnalysis::decl()));
    out.push('\n');

    out.push_str(&exported_decl(&ExportFormat::decl()));
    out.push('\n');
    out.push_str(&exported_decl(&ExportField::decl()));
    out.push('\n');
    out.push_str(&exported_decl(&ExportRequest::decl()));
    out.push('\n');

    out
}

/// 将 `ts-rs` 的声明文本转换为可被 TS 模块导出的形式（补齐 `export` 关键字）。
fn exported_decl(decl: &str) -> String {
    let trimmed = decl.trim_start();
    if trimmed.starts_with("type ") {
        return decl.replacen("type ", "export type ", 1);
    }
    if trimmed.starts_with("interface ") {
        return decl.replacen("interface ", "export interface ", 1);
    }
    if trimmed.starts_with("enum ") {
        return decl.replacen("enum ", "export enum ", 1);
    }
    format!("export {decl}")
}

/// 程序入口：生成并写入类型文件。
fn main() {
    let args: Vec<String> = std::env::args().collect();
    let out_path = resolve_out_path(&args);
    let content = build_types_file();

    if let Some(parent) = out_path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }

    std::fs::write(&out_path, content).expect("写入 TypeScript 类型文件失败");
    println!("已生成：{}", out_path.to_string_lossy());
}
