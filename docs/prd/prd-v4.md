# 番茄钟 PRD v4

## 版本概述

新增白噪音/专注音乐、番茄完成动画、番茄中断记录三大功能，提升专注体验和数据洞察能力。

## 功能需求

### 1. 白噪音/专注音乐

**功能描述：**

- 专注时播放背景音效，帮助用户进入心流状态
- 支持自定义导入
- 音量独立控制，与系统音量分离
- 计时结束时自动淡出

**自定义音频：**

- 支持导入本地 mp3/wav/ogg/flac 文件
- 文件复制到 `%APPDATA%/pomodoro-technique/audio/` 目录
- 不限制自定义音频数量
- 支持删除自定义音频（预设不可删除）

**播放控制：**

- 音量滑块：0-100%
- 播放/暂停按钮
- 切换音效下拉菜单
- 「随番茄自动播放」开关：开启后，番茄开始时自动播放，休息时暂停

**淡出效果：**

- 计时结束前 5 秒开始淡出
- 淡出时长 3 秒
- 淡出后自动暂停

**交互入口：**

- 主界面计时器下方新增音效控制栏
- 设置页面新增「音效设置」区域（管理自定义音频）

**Tauri 实现：**

- 使用 `rodio` crate 播放音频
- 音频文件打包到 resources 目录
- 通过 Tauri 命令控制播放/暂停/音量

### 2. 番茄完成动画

**功能描述：**

- 番茄完成时播放庆祝动画，增强仪式感
- 支持多种动画效果
- 连续完成时显示 combo 效果
- 可在设置中关闭

**动画类型：**
| 动画名 | 触发条件 | 效果描述 |
|--------|----------|----------|
| 粒子爆发 | 单个番茄完成 | 从计时器中心向外扩散的彩色粒子 |
| 烟花 | 达成每日目标 | 屏幕多点烟花绽放 |
| Combo | 连续完成 2+ 个 | 显示 `x2`、`x3` 等连击数字 |

**动画规格：**

- 时长：1.5-3 秒
- 不阻塞用户操作
- 支持点击跳过
- 使用 CSS 动画 + Canvas 实现

**Combo 机制：**

- 连续完成定义：两个番茄间隔 ≤ 休息时长 + 5 分钟
- Combo 数显示在计时器右上角
- 中断后 Combo 重置为 0

**设置选项：**

- 「启用完成动画」开关（默认开启）
- 「启用 Combo 显示」开关（默认开启）
- 「动画强度」：简约 / 标准 / 华丽

**前端实现：**

- 新增 `CompletionAnimation.svelte` 组件
- 使用 `canvas-confetti` 库实现粒子效果
- Combo 数使用 CSS transform 动画

### 3. 番茄中断记录

**功能描述：**

- 记录番茄被中断的情况，帮助用户识别干扰模式
- 中断时可选填中断原因
- 统计中断频率、时段、原因分布
- 在历史记录和分析页面展示

**中断定义：**

- 工作阶段进行中，用户点击「重置」或「跳过」
- 工作阶段进行中，用户关闭应用
- 不包括休息阶段的中断

**中断记录字段：**

```typescript
interface InterruptionRecord {
  /** 中断时间（ISO 8601）。 */
  timestamp: string;
  /** 中断时剩余秒数。 */
  remainingSeconds: number;
  /** 已专注秒数。 */
  focusedSeconds: number;
  /** 中断原因（用户填写，可为空）。 */
  reason: string;
  /** 中断类型。 */
  type: "reset" | "skip" | "quit";
  /** 当时的任务标签。 */
  tag: string;
}
```

**预设中断原因：**

- 紧急事务
- 电话/消息
- 会议
- 休息需求
- 其他（自定义输入）

**交互流程：**

1. 用户在工作阶段点击「重置」或「跳过」
2. 弹出中断确认弹窗
3. 显示已专注时长，询问是否记录中断
4. 可选填中断原因（下拉 + 自定义输入）
5. 确认后记录中断并执行操作

**中断统计分析：**

- 中断频率：每日/每周平均中断次数
- 中断时段：哪个小时中断最多
- 原因分布：饼图展示各原因占比
- 中断率：中断番茄数 / 总开始番茄数
- 平均专注时长：中断前平均已专注时间

**展示位置：**

- 历史记录页面：中断记录以不同颜色标记
- 专注分析页面：新增「中断分析」卡片
- 今日统计：显示今日中断次数

**设置选项：**

- 「记录中断」开关（默认开启）
- 「中断时弹窗确认」开关（默认开启）

## 数据结构变更

```typescript
// Settings 新增字段
interface Settings {
  // ... 现有字段 ...

  /** 音效设置 */
  audio: {
    /** 是否启用音效。 */
    enabled: boolean;
    /** 当前选中的音效 id。 */
    currentAudioId: string;
    /** 音量（0-100）。 */
    volume: number;
    /** 是否随番茄自动播放。 */
    autoPlay: boolean;
  };

  /** 动画设置 */
  animation: {
    /** 是否启用完成动画。 */
    enabled: boolean;
    /** 是否启用 Combo 显示。 */
    comboEnabled: boolean;
    /** 动画强度：minimal / standard / fancy。 */
    intensity: "minimal" | "standard" | "fancy";
  };

  /** 中断设置 */
  interruption: {
    /** 是否记录中断。 */
    enabled: boolean;
    /** 中断时是否弹窗确认。 */
    confirmOnInterrupt: boolean;
  };
}

// 自定义音频条目
interface CustomAudio {
  /** 音频 id（uuid）。 */
  id: string;
  /** 显示名称。 */
  name: string;
  /** 文件名（存储在 audio 目录）。 */
  fileName: string;
  /** 是否内置。 */
  builtin: boolean;
}

// 中断记录
interface InterruptionRecord {
  timestamp: string;
  remainingSeconds: number;
  focusedSeconds: number;
  reason: string;
  type: "reset" | "skip" | "quit";
  tag: string;
}

// 某一天的中断集合
interface InterruptionDay {
  date: string;
  records: InterruptionRecord[];
}

// AppData 新增字段
interface AppData {
  // ... 现有字段 ...

  /** 自定义音频列表。 */
  customAudios: CustomAudio[];
  /** 中断记录。 */
  interruptions: InterruptionDay[];
  /** 当前 Combo 数（运行时状态，可选持久化）。 */
  currentCombo: number;
  /** 累计完成番茄总数（用于里程碑）。 */
  totalPomodoros: number;
}
```

## 新增 Tauri 命令

| 命令                     | 参数                              | 返回                 | 说明                        |
| ------------------------ | --------------------------------- | -------------------- | --------------------------- |
| `audio_play`             | `audio_id: string`                | `bool`               | 播放指定音效                |
| `audio_pause`            | -                                 | `bool`               | 暂停播放                    |
| `audio_set_volume`       | `volume: u8`                      | `bool`               | 设置音量（0-100）           |
| `audio_import`           | `file_path: string, name: string` | `CustomAudio`        | 导入自定义音频              |
| `audio_delete`           | `audio_id: string`                | `bool`               | 删除自定义音频              |
| `audio_list`             | -                                 | `CustomAudio[]`      | 获取音频列表（内置+自定义） |
| `record_interruption`    | `reason: string, type: string`    | `InterruptionRecord` | 记录中断                    |
| `get_interruption_stats` | `range: DateRange`                | `InterruptionStats`  | 获取中断统计                |
| `get_combo`              | -                                 | `u32`                | 获取当前 Combo 数           |
| `get_total_pomodoros`    | -                                 | `u64`                | 获取累计番茄总数            |

## 新增前端事件

| 事件名               | 载荷                                                          | 说明                       |
| -------------------- | ------------------------------------------------------------- | -------------------------- |
| `pomodoro-completed` | `{ combo: number, total: number, dailyGoalReached: boolean }` | 番茄完成，触发动画         |
| `milestone-reached`  | `{ milestone: number }`                                       | 达成里程碑（100/500/1000） |

## 新增前端组件

| 组件                         | 说明               |
| ---------------------------- | ------------------ |
| `AudioControl.svelte`        | 主界面音效控制栏   |
| `AudioSettings.svelte`       | 设置页音效管理区域 |
| `CompletionAnimation.svelte` | 完成动画容器       |
| `ComboDisplay.svelte`        | Combo 数显示       |
| `InterruptionModal.svelte`   | 中断确认弹窗       |
| `InterruptionStats.svelte`   | 中断统计卡片       |

## 资源文件

```
src-tauri/
  resources/
    audio/
      rain.mp3
      cafe.mp3
      forest.mp3
      ocean.mp3
      white-noise.mp3
```

## 依赖新增

**Rust (Cargo.toml):**

```toml
rodio = "0.19"  # 音频播放
```

**前端 (package.json):**

```json
{
  "canvas-confetti": "^1.9.0"
}
```

## 实现优先级

1. **P0 - 番茄中断记录**：数据价值高，实现简单
2. **P1 - 番茄完成动画**：提升体验，纯前端实现
3. **P2 - 白噪音/专注音乐**：需要音频资源和 Rust 音频库集成

## 第四版范围外（后续迭代）

- 音效混合（同时播放多个音效）
- 自定义动画主题
- 中断原因智能分析（识别高频干扰源）
- 番茄专注评分（基于中断率计算）
