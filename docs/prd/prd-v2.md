# 番茄钟 PRD v2

## 版本概述

基于 v1 核心功能，新增数据分析、目标管理、窗口模式等增强功能，提升用户专注效率和使用体验。

## 新增功能需求

### 1. 历史记录查看

**功能描述：**
- 展示历史番茄完成记录（v1 已有 `history` 数据结构，本版本增加 UI）
- 支持按日期筛选（日/周/月视图）
- 按标签分组统计
- 显示每条记录的开始时间、时长、标签

**交互流程：**
1. 主界面新增「历史记录」入口
2. 默认显示本周记录
3. 可切换日/周/月视图
4. 点击具体日期展开当日详情

**UI 设计：**
- 日历热力图展示每日完成数量
- 列表视图显示详细记录
- 支持滚动加载历史数据

### 2. 番茄目标设定

**功能描述：**
- 设定每日番茄目标数量
- 设定每周番茄目标数量
- 目标达成时触发通知
- 主界面显示目标进度

**交互流程：**
1. 设置页面新增「目标设定」区域
2. 输入每日/每周目标数量（0 表示不设目标）
3. 主界面显示进度条：`今日 3/8`、`本周 15/40`

**目标提醒：**
- 达成 50% 时轻提示
- 达成 100% 时庆祝通知
- 超额完成时显示超额数量

### 3. 专注时段分析

**功能描述：**
- 分析用户历史数据，识别高效专注时段
- 按小时统计番茄完成分布
- 生成个人专注效率报告

**分析维度：**
- 时段分布：0-6 / 6-12 / 12-18 / 18-24
- 星期分布：周一至周日
- 标签效率：各标签平均专注时长

**展示方式：**
- 柱状图展示时段分布
- 热力图展示周/时段交叉分析
- 文字总结：「你在上午 9-11 点专注效率最高」

### 4. 黑名单模板

**功能描述：**
- 预设多套黑名单配置
- 支持快速切换模板
- 内置默认模板 + 用户自定义模板

**内置模板：**
| 模板名 | 默认黑名单 |
|--------|-----------|
| 工作模式 | 微信、QQ、抖音、B站 |
| 学习模式 | 微信、QQ、游戏平台、视频网站 |
| 深度专注 | 所有社交 + 娱乐 + 浏览器 |

**交互流程：**
1. 黑名单管理页面新增「模板」下拉选择
2. 选择模板后自动填充黑名单
3. 可基于模板修改后「另存为新模板」
4. 支持删除自定义模板（内置模板不可删除）

### 5. 窗口置顶/迷你模式

**功能描述：**
- 窗口置顶：主窗口始终显示在最前
- 迷你模式：小窗口仅显示倒计时

**迷你模式规格：**
- 窗口尺寸：200×80 px
- 显示内容：倒计时 + 阶段指示器
- 可拖拽定位
- 双击恢复主窗口

**交互流程：**
1. 标题栏新增「置顶」按钮（图钉图标）
2. 标题栏新增「迷你模式」按钮
3. 迷你窗口右键菜单：恢复/暂停/退出

**Tauri 实现：**
- 置顶：`set_always_on_top(true)`
- 迷你模式：创建新窗口或调整主窗口尺寸

### 6. 导出功能

**功能描述：**
- 导出历史记录为 CSV/JSON 格式
- 支持选择导出时间范围
- 支持选择导出字段

**导出字段：**
```
日期, 开始时间, 结束时间, 时长(分钟), 标签, 阶段类型
```

**交互流程：**
1. 历史记录页面新增「导出」按钮
2. 弹窗选择：时间范围 + 格式（CSV/JSON）
3. 调用系统文件保存对话框
4. 导出完成后提示成功

**文件格式示例：**

CSV:
```csv
date,start_time,end_time,duration,tag,phase
2025-01-15,09:00,09:25,25,工作,work
2025-01-15,09:30,09:35,5,工作,shortBreak
```

JSON:
```json
{
  "exportDate": "2025-01-20",
  "range": { "from": "2025-01-01", "to": "2025-01-20" },
  "records": [
    {
      "date": "2025-01-15",
      "startTime": "09:00",
      "endTime": "09:25",
      "duration": 25,
      "tag": "工作",
      "phase": "work"
    }
  ]
}
```

### 7. 日志系统

**功能描述：**
- 完整的应用日志记录
- 支持多级别日志（DEBUG/INFO/WARN/ERROR）
- 日志文件自动轮转
- 提供日志查看/导出入口

**日志内容：**
| 模块 | 记录内容 |
|------|---------|
| 计时器 | 开始/暂停/重置/跳过/完成 |
| 黑名单 | 进程终止成功/失败/权限不足 |
| 存储 | 数据读取/写入/错误 |
| 窗口 | 模式切换/置顶状态变更 |
| 系统 | 启动/退出/崩溃信息 |

**日志格式：**
```
[2025-01-15 09:00:00.123] [INFO] [timer] 番茄计时开始，时长=25分钟，标签=工作
[2025-01-15 09:00:01.456] [DEBUG] [blacklist] 尝试终止进程: WeChat.exe (PID: 1234)
[2025-01-15 09:00:01.789] [WARN] [blacklist] 进程终止失败，权限不足: WeChat.exe
```

**存储策略：**
- 日志目录：`%APPDATA%/pomodoro-technique/logs/`
- 单文件上限：10 MB
- 保留最近 7 个日志文件
- 文件命名：`pomodoro-YYYY-MM-DD.log`

**Rust 实现：**
- 使用 `tracing` + `tracing-subscriber` crate
- 配置 `RollingFileAppender` 实现日志轮转

**用户入口：**
- 设置页面新增「查看日志」按钮
- 点击后打开日志目录（调用系统文件管理器）

## 数据结构变更

```json
{
  "settings": {
    "pomodoro": 25,
    "shortBreak": 5,
    "longBreak": 15,
    "longBreakInterval": 4,
    "dailyGoal": 8,
    "weeklyGoal": 40,
    "alwaysOnTop": false
  },
  "blacklist": [...],
  "blacklistTemplates": [
    {
      "id": "work",
      "name": "工作模式",
      "builtin": true,
      "processes": [
        { "name": "WeChat.exe", "displayName": "微信" }
      ]
    },
    {
      "id": "custom-1",
      "name": "我的模板",
      "builtin": false,
      "processes": [...]
    }
  ],
  "activeTemplateId": "work",
  "tags": [...],
  "history": [
    {
      "date": "2025-01-15",
      "records": [
        {
          "tag": "工作",
          "startTime": "09:00",
          "endTime": "09:25",
          "duration": 25,
          "phase": "work"
        }
      ]
    }
  ]
}
```

## 新增 Tauri 命令

| 命令 | 参数 | 返回 | 说明 |
|------|------|------|------|
| `get_history` | `range: DateRange` | `HistoryRecord[]` | 获取指定范围历史 |
| `get_focus_analysis` | `range: DateRange` | `FocusAnalysis` | 获取专注分析数据 |
| `set_goals` | `daily: u32, weekly: u32` | `Settings` | 设置目标 |
| `get_templates` | - | `Template[]` | 获取所有模板 |
| `save_template` | `template: Template` | `Template` | 保存模板 |
| `delete_template` | `id: string` | `bool` | 删除模板 |
| `apply_template` | `id: string` | `Blacklist` | 应用模板 |
| `set_always_on_top` | `enabled: bool` | `bool` | 设置置顶 |
| `set_mini_mode` | `enabled: bool` | `bool` | 切换迷你模式 |
| `export_history` | `range: DateRange, format: string` | `string` | 导出历史（返回文件路径） |
| `open_log_dir` | - | `bool` | 打开日志目录 |

## 新增前端路由/组件

| 路由/组件 | 说明 |
|-----------|------|
| `/history` | 历史记录页面 |
| `HistoryCalendar.svelte` | 日历热力图组件 |
| `FocusAnalysis.svelte` | 专注分析图表组件 |
| `TemplateSelector.svelte` | 模板选择器组件 |
| `ExportModal.svelte` | 导出弹窗组件 |
| `MiniWindow.svelte` | 迷你模式窗口 |
| `GoalProgress.svelte` | 目标进度条组件 |

## 第二版范围外（后续迭代）

- 云同步
- 快捷键支持
- 白噪音/专注音乐
- 番茄完成动画
- 多设备数据同步
- 团队协作功能
