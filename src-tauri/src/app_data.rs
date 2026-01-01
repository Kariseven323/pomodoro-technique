//! PRD 约定的数据结构（settings / blacklist / tags / history）。

use serde::{Deserialize, Serialize};

/// Store 文件路径（同时被 Rust 与前端 guest bindings 兼容）。
pub const STORE_PATH: &str = "pomodoro-data.json";

/// Store 内部主键（保存整棵 `AppData`）。
pub const STORE_KEY: &str = "appData";

/// 番茄钟设置。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    /// 工作时长（分钟）。
    pub pomodoro: u32,
    /// 短休息时长（分钟）。
    pub short_break: u32,
    /// 长休息时长（分钟）。
    pub long_break: u32,
    /// 长休息间隔（每 N 个番茄触发）。
    pub long_break_interval: u32,
}

impl Default for Settings {
    /// PRD 默认设置：25/5/15/4。
    fn default() -> Self {
        Self {
            pomodoro: 25,
            short_break: 5,
            long_break: 15,
            long_break_interval: 4,
        }
    }
}

/// 黑名单条目（以进程名为主键）。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct BlacklistItem {
    /// 进程名（例如 `WeChat.exe`）。
    pub name: String,
    /// 展示名（例如 `微信`）。
    pub display_name: String,
}

/// 单条历史记录（仅记录完成的“工作”阶段）。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HistoryRecord {
    /// 任务标签。
    pub tag: String,
    /// 开始时间（HH:mm）。
    pub start_time: String,
    /// 本次番茄时长（分钟）。
    pub duration: u32,
}

/// 某一天的历史集合。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HistoryDay {
    /// 日期（YYYY-MM-DD）。
    pub date: String,
    /// 当日记录。
    pub records: Vec<HistoryRecord>,
}

/// 应用持久化数据根对象。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppData {
    /// 用户设置。
    pub settings: Settings,
    /// 进程黑名单。
    pub blacklist: Vec<BlacklistItem>,
    /// 历史标签。
    pub tags: Vec<String>,
    /// 历史记录（按日分组）。
    pub history: Vec<HistoryDay>,
}

impl Default for AppData {
    /// PRD 初始数据：默认 settings + 默认 tags + 空 blacklist/history。
    fn default() -> Self {
        Self {
            settings: Settings::default(),
            blacklist: Vec::new(),
            tags: vec!["工作".to_string(), "学习".to_string(), "阅读".to_string(), "写作".to_string()],
            history: Vec::new(),
        }
    }
}

