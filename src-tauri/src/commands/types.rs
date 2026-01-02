//! 前端命令入参/出参类型（用于 IPC 序列化）。

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::app_data::DateRange;
use crate::timer::TimerSnapshot;

/// 前端初始化所需的完整快照（持久化数据 + 计时器状态）。
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct AppSnapshot {
    /// 持久化数据（settings/blacklist/tags/history）。
    pub data: crate::app_data::AppData,
    /// 计时器状态快照。
    pub timer: TimerSnapshot,
}

/// 应用数据根目录路径信息（用于设置页展示与“打开文件夹”入口）。
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct StorePaths {
    /// 数据根目录路径（统一入口，可用于打开文件夹）。
    pub store_dir_path: String,
}

/// 导出格式（CSV/JSON）。
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub enum ExportFormat {
    /// CSV（逗号分隔）。
    Csv,
    /// JSON（结构化）。
    Json,
}

/// 导出字段（用于“自选导出字段”）。
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub enum ExportField {
    /// 日期（YYYY-MM-DD）。
    Date,
    /// 开始时间（HH:mm）。
    StartTime,
    /// 结束时间（HH:mm）。
    EndTime,
    /// 时长（分钟）。
    Duration,
    /// 标签。
    Tag,
    /// 阶段类型（work/shortBreak/longBreak）。
    Phase,
    /// 备注（PRD v2 新增，可选导出）。
    Remark,
}

/// 导出请求参数。
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct ExportRequest {
    /// 导出范围。
    pub range: DateRange,
    /// 导出格式。
    pub format: ExportFormat,
    /// 导出字段（为空则导出默认字段集）。
    #[serde(default)]
    pub fields: Vec<ExportField>,
}
