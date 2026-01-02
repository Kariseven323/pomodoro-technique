//! PRD 约定的数据结构（settings / blacklist / tags / history）。

use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// Store 文件名（最终路径由后端根据平台解析到统一的数据根目录下）。
pub const STORE_FILE_NAME: &str = "pomodoro-data.json";

/// Store 内部主键（保存整棵 `AppData`）。
pub const STORE_KEY: &str = "appData";

/// 计时器/历史记录阶段（与前端 `Phase` 类型对齐）。
#[derive(Debug, Copy, Clone, Serialize, Deserialize, PartialEq, Eq, TS)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub enum Phase {
    /// 工作（番茄）。
    Work,
    /// 短休息。
    ShortBreak,
    /// 长休息。
    LongBreak,
}

impl Default for Phase {
    /// 默认阶段：工作（用于旧数据缺失字段时的兼容回填）。
    fn default() -> Self {
        Self::Work
    }
}

/// 番茄钟设置。
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct Settings {
    /// 工作时长（分钟）。
    pub pomodoro: u32,
    /// 短休息时长（分钟）。
    pub short_break: u32,
    /// 长休息时长（分钟）。
    pub long_break: u32,
    /// 长休息间隔（每 N 个番茄触发）。
    pub long_break_interval: u32,
    /// 是否启用“休息结束后自动进入工作倒计时”（连续番茄模式）。
    #[serde(default)]
    pub auto_continue_enabled: bool,
    /// 连续番茄数量：在该次数内，休息结束后自动开始下一次工作倒计时。
    #[serde(default = "default_auto_continue_pomodoros")]
    pub auto_continue_pomodoros: u32,
    /// 每日目标番茄数量（0 表示不设目标）。
    #[serde(default = "default_daily_goal")]
    pub daily_goal: u32,
    /// 每周目标番茄数量（0 表示不设目标）。
    #[serde(default = "default_weekly_goal")]
    pub weekly_goal: u32,
    /// 窗口是否置顶（主窗口）。
    #[serde(default)]
    pub always_on_top: bool,
}

/// 默认连续番茄数量（用于旧版本数据缺失字段时的兼容回填）。
fn default_auto_continue_pomodoros() -> u32 {
    4
}

/// 默认每日目标（PRD v2：8）。
fn default_daily_goal() -> u32 {
    8
}

/// 默认每周目标（PRD v2：40）。
fn default_weekly_goal() -> u32 {
    40
}

impl Default for Settings {
    /// PRD 默认设置：25/5/15/4。
    fn default() -> Self {
        Self {
            pomodoro: 25,
            short_break: 5,
            long_break: 15,
            long_break_interval: 4,
            auto_continue_enabled: false,
            auto_continue_pomodoros: 4,
            daily_goal: default_daily_goal(),
            weekly_goal: default_weekly_goal(),
            always_on_top: false,
        }
    }
}

/// 黑名单条目（以进程名为主键）。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, TS)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct BlacklistItem {
    /// 进程名（例如 `WeChat.exe`）。
    pub name: String,
    /// 展示名（例如 `微信`）。
    pub display_name: String,
}

/// 黑名单模板（可内置/可自定义）。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, TS)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct BlacklistTemplate {
    /// 模板 id（内置模板为固定值，自定义模板可为 uuid/自定义字符串）。
    pub id: String,
    /// 模板名称。
    pub name: String,
    /// 是否为内置模板（内置模板不可删除）。
    pub builtin: bool,
    /// 模板包含的黑名单进程集合。
    pub processes: Vec<BlacklistItem>,
}

/// 单条历史记录（仅记录完成的“工作”阶段）。
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct HistoryRecord {
    /// 任务标签。
    pub tag: String,
    /// 开始时间（HH:mm）。
    pub start_time: String,
    /// 结束时间（HH:mm；旧数据可能缺失，前端可按 `start_time + duration` 推导展示）。
    #[serde(default)]
    pub end_time: Option<String>,
    /// 本次番茄时长（分钟）。
    pub duration: u32,
    /// 阶段类型（用于导出/分析；当前仅记录工作阶段）。
    #[serde(default)]
    pub phase: Phase,
    /// 备注（完成后可填写，也可在历史中编辑）。
    #[serde(default)]
    pub remark: String,
}

/// 某一天的历史集合。
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct HistoryDay {
    /// 日期（YYYY-MM-DD）。
    pub date: String,
    /// 当日记录。
    pub records: Vec<HistoryRecord>,
}

/// 日期范围（闭区间）：`from <= date <= to`。
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct DateRange {
    /// 起始日期（YYYY-MM-DD）。
    pub from: String,
    /// 结束日期（YYYY-MM-DD）。
    pub to: String,
}

/// 应用持久化数据根对象。
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct AppData {
    /// 用户设置。
    pub settings: Settings,
    /// 进程黑名单。
    pub blacklist: Vec<BlacklistItem>,
    /// 黑名单模板列表（包含内置模板与自定义模板）。
    #[serde(default)]
    pub blacklist_templates: Vec<BlacklistTemplate>,
    /// 当前启用的模板 id 列表（支持同时启用多套模板）。
    #[serde(default)]
    pub active_template_ids: Vec<String>,
    /// 兼容字段：旧/示例数据中的单一激活模板（用于自动迁移到 `active_template_ids`）。
    #[serde(default)]
    pub active_template_id: Option<String>,
    /// 历史标签。
    pub tags: Vec<String>,
    /// 历史记录（按日分组）。
    pub history: Vec<HistoryDay>,
    /// 调试历史记录（仅开发环境使用，与正式数据隔离）。
    #[serde(default)]
    pub history_dev: Vec<HistoryDay>,
}

impl Default for AppData {
    /// PRD 初始数据：默认 settings + 默认 tags + 空 blacklist/history。
    fn default() -> Self {
        let templates = builtin_templates();
        let active = vec!["work".to_string()];
        let blacklist = templates
            .iter()
            .find(|t| t.id == "work")
            .map(|t| t.processes.clone())
            .unwrap_or_default();

        Self {
            settings: Settings::default(),
            blacklist,
            blacklist_templates: templates,
            active_template_ids: active.clone(),
            active_template_id: active.first().cloned(),
            tags: vec![
                "工作".to_string(),
                "学习".to_string(),
                "阅读".to_string(),
                "写作".to_string(),
            ],
            history: Vec::new(),
            history_dev: Vec::new(),
        }
    }
}

impl AppData {
    /// 将旧版本数据迁移到 v2 结构（回填缺失字段、补齐内置模板、兼容单模板字段）。
    pub fn migrate_v2(&mut self) -> bool {
        let mut changed = false;

        if self.blacklist_templates.is_empty() {
            self.blacklist_templates = builtin_templates();
            changed = true;
        }

        if self.active_template_ids.is_empty() {
            if let Some(id) = self.active_template_id.clone() {
                self.active_template_ids = vec![id];
                changed = true;
            }
        }

        // 保持 `active_template_id` 与数组一致，便于示例/兼容。
        let first = self.active_template_ids.first().cloned();
        if self.active_template_id != first {
            self.active_template_id = first;
            changed = true;
        }

        changed
    }
}

/// 构建 PRD v2 内置黑名单模板列表。
fn builtin_templates() -> Vec<BlacklistTemplate> {
    vec![
        BlacklistTemplate {
            id: "work".to_string(),
            name: "工作模式".to_string(),
            builtin: true,
            processes: vec![
                BlacklistItem {
                    name: "WeChat.exe".to_string(),
                    display_name: "微信".to_string(),
                },
                BlacklistItem {
                    name: "QQ.exe".to_string(),
                    display_name: "QQ".to_string(),
                },
                BlacklistItem {
                    name: "Douyin.exe".to_string(),
                    display_name: "抖音".to_string(),
                },
                BlacklistItem {
                    name: "Bilibili.exe".to_string(),
                    display_name: "B站".to_string(),
                },
            ],
        },
        BlacklistTemplate {
            id: "study".to_string(),
            name: "学习模式".to_string(),
            builtin: true,
            processes: vec![
                BlacklistItem {
                    name: "WeChat.exe".to_string(),
                    display_name: "微信".to_string(),
                },
                BlacklistItem {
                    name: "QQ.exe".to_string(),
                    display_name: "QQ".to_string(),
                },
                BlacklistItem {
                    name: "Steam.exe".to_string(),
                    display_name: "游戏平台".to_string(),
                },
                BlacklistItem {
                    name: "Bilibili.exe".to_string(),
                    display_name: "视频网站".to_string(),
                },
            ],
        },
        BlacklistTemplate {
            id: "deep".to_string(),
            name: "深度专注".to_string(),
            builtin: true,
            processes: vec![
                BlacklistItem {
                    name: "WeChat.exe".to_string(),
                    display_name: "微信".to_string(),
                },
                BlacklistItem {
                    name: "QQ.exe".to_string(),
                    display_name: "QQ".to_string(),
                },
                BlacklistItem {
                    name: "Douyin.exe".to_string(),
                    display_name: "抖音".to_string(),
                },
                BlacklistItem {
                    name: "Bilibili.exe".to_string(),
                    display_name: "B站".to_string(),
                },
                BlacklistItem {
                    name: "chrome.exe".to_string(),
                    display_name: "浏览器（Chrome）".to_string(),
                },
                BlacklistItem {
                    name: "msedge.exe".to_string(),
                    display_name: "浏览器（Edge）".to_string(),
                },
                BlacklistItem {
                    name: "firefox.exe".to_string(),
                    display_name: "浏览器（Firefox）".to_string(),
                },
            ],
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    /// v2 迁移：当模板列表缺失时应补齐内置模板，并修复激活模板字段。
    #[test]
    fn migrate_v2_fills_templates_and_active_ids() {
        let mut data = AppData {
            blacklist_templates: Vec::new(),
            active_template_ids: Vec::new(),
            active_template_id: Some("work".to_string()),
            ..AppData::default()
        };

        let changed = data.migrate_v2();
        assert!(changed);
        assert!(!data.blacklist_templates.is_empty());
        assert!(!data.active_template_ids.is_empty());
        assert_eq!(data.active_template_ids[0], "work");
        assert_eq!(data.active_template_id.as_deref(), Some("work"));
    }

    /// v2 迁移：应保证 `active_template_id` 与数组首项一致，便于兼容读取。
    #[test]
    fn migrate_v2_keeps_active_template_id_in_sync() {
        let mut data = AppData {
            active_template_ids: vec!["deep".to_string()],
            active_template_id: Some("work".to_string()),
            ..AppData::default()
        };

        let changed = data.migrate_v2();
        assert!(changed);
        assert_eq!(data.active_template_id.as_deref(), Some("deep"));
    }

    /// `Settings::default`：默认值应符合 PRD 约定（25/5/15/4 + 目标值）。
    #[test]
    fn settings_default_matches_prd() {
        let s = Settings::default();
        assert_eq!(s.pomodoro, 25);
        assert_eq!(s.short_break, 5);
        assert_eq!(s.long_break, 15);
        assert_eq!(s.long_break_interval, 4);
        assert_eq!(s.auto_continue_enabled, false);
        assert_eq!(s.auto_continue_pomodoros, 4);
        assert_eq!(s.daily_goal, 8);
        assert_eq!(s.weekly_goal, 40);
        assert_eq!(s.always_on_top, false);
    }

    /// `builtin_templates`：应包含固定的内置模板集合，且均标记为 builtin。
    #[test]
    fn builtin_templates_have_expected_ids_and_flags() {
        let templates = builtin_templates();
        assert!(templates.len() >= 3);
        assert!(templates.iter().all(|t| t.builtin));

        let ids: std::collections::BTreeSet<String> =
            templates.iter().map(|t| t.id.clone()).collect();
        assert_eq!(ids.len(), templates.len());
        assert!(ids.contains("work"));
        assert!(ids.contains("study"));
        assert!(ids.contains("deep"));
    }

    /// `AppData::default`：应填充默认模板/默认激活模板，并回填默认黑名单与标签。
    #[test]
    fn app_data_default_initializes_templates_blacklist_and_tags() {
        let data = AppData::default();

        assert!(!data.blacklist_templates.is_empty());
        assert_eq!(data.active_template_ids.first().map(|s| s.as_str()), Some("work"));
        assert_eq!(data.active_template_id.as_deref(), Some("work"));

        let work_template = data
            .blacklist_templates
            .iter()
            .find(|t| t.id == "work")
            .expect("work 模板必须存在");
        assert_eq!(data.blacklist, work_template.processes);

        assert_eq!(
            data.tags,
            vec!["工作".to_string(), "学习".to_string(), "阅读".to_string(), "写作".to_string()]
        );
        assert!(data.history.is_empty());
    }

    /// `Phase::default`：应回退为工作阶段（用于旧数据缺失字段时兼容）。
    #[test]
    fn phase_default_is_work() {
        assert_eq!(Phase::default(), Phase::Work);
    }

    /// `default_auto_continue_pomodoros`：应返回 PRD 约定的默认连续番茄数量。
    #[test]
    fn default_auto_continue_pomodoros_matches_prd() {
        assert_eq!(default_auto_continue_pomodoros(), 4);
    }
}
