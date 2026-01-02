# 测试覆盖率提升计划（目标 90%，排除 UI）

## 现状分析

当前项目测试覆盖情况：

- **Rust 后端**：仅 `timer/runtime.rs`、`timer/stats.rs`、`timer/validation.rs`、`app_data.rs` 有单元测试
- **TypeScript 前端**：无测试配置，无测试文件
- **测试框架**：Rust 使用内置 `#[cfg(test)]`，前端未配置 vitest

---

## 一、Rust 后端测试（src-tauri/src/）

### 1.1 analysis.rs - 专注分析模块

| 函数                 | 优先级 | 测试点                                    |
| -------------------- | ------ | ----------------------------------------- |
| `get_focus_analysis` | P0     | 空数据、单日数据、跨周数据、边界日期      |
| `parse_range`        | P0     | 合法范围、from > to、格式错误             |
| `parse_hour`         | P1     | 合法时间、边界值（00:00/23:59）、非法格式 |
| `period_index`       | P1     | 各时段边界（0/5/6/11/12/17/18/23）        |
| `weekday_to_index`   | P1     | 周一至周日映射                            |
| `build_summary`      | P1     | 空数据、单峰、多峰、边界小时              |
| `time_range_label`   | P2     | 凌晨/上午/下午/晚上、12点边界             |

### 1.2 timer/notification.rs - 通知模块

| 函数                             | 优先级 | 测试点                                            |
| -------------------------------- | ------ | ------------------------------------------------- |
| `notify_phase_end`               | P0     | Work/ShortBreak/LongBreak 结束、自动开始/手动开始 |
| `notify_goal_progress_if_needed` | P0     | 50%阈值、100%阈值、目标为0、跨阈值                |
| `phase_preview`                  | P1     | 各阶段预告文案、自动/手动前缀                     |

### 1.3 processes/termination.rs - 进程终止模块

| 函数                     | 优先级 | 测试点                      |
| ------------------------ | ------ | --------------------------- |
| `kill_names_best_effort` | P0     | 空列表、单进程、多进程      |
| `eq_process_name`        | P1     | 大小写敏感/不敏感（跨平台） |

> 注：`kill_pid`/`kill_by_name` 涉及系统调用，建议通过 mock 或集成测试覆盖

### 1.4 processes/enumeration.rs - 进程枚举模块

| 函数             | 优先级 | 测试点                           |
| ---------------- | ------ | -------------------------------- |
| `list_processes` | P2     | 返回结构正确性（需 mock System） |

> 注：图标提取为 Windows 特定，可跳过或条件编译测试

### 1.5 app_data.rs - 数据结构模块（已有部分测试）

| 函数                | 优先级 | 测试点             |
| ------------------- | ------ | ------------------ |
| `Settings::default` | P2     | 默认值符合 PRD     |
| `AppData::default`  | P2     | 默认模板、默认标签 |
| `builtin_templates` | P2     | 模板数量、内置标记 |

### 1.6 timer/runtime.rs - 计时器运行态（已有部分测试）

| 函数                  | 优先级 | 测试点                 |
| --------------------- | ------ | ---------------------- |
| `TimerRuntime::new`   | P1     | 初始状态、空标签回退   |
| `start`               | P1     | 重复启动幂等、锁定标记 |
| `pause`               | P1     | 暂停状态               |
| `reset`               | P1     | 重置后状态             |
| `skip`                | P1     | 跳过不写历史、阶段切换 |
| `blacklist_locked`    | P1     | 锁定条件               |
| `set_current_tag`     | P2     | 空白标签规范化         |
| `snapshot_with_clock` | P2     | 快照字段完整性         |

### 1.7 errors.rs - 错误类型

| 测试点                    | 优先级 |
| ------------------------- | ------ |
| 各错误变体的 Display 输出 | P2     |

### 1.8 commands/\*.rs - Tauri 命令层

| 模块                     | 优先级 | 测试策略             |
| ------------------------ | ------ | -------------------- |
| `commands/validation.rs` | P1     | 参数校验逻辑         |
| `commands/settings.rs`   | P1     | 设置读写             |
| `commands/blacklist.rs`  | P1     | 黑名单增删、锁定检查 |
| `commands/tags.rs`       | P2     | 标签增删             |
| `commands/history.rs`    | P2     | 历史查询、备注更新   |
| `commands/timer.rs`      | P2     | 计时器控制           |
| `commands/export.rs`     | P2     | CSV 导出格式         |
| `commands/templates.rs`  | P2     | 模板管理             |

> 注：commands 层建议通过集成测试或 mock AppState 测试

---

## 二、TypeScript 前端测试（src/lib/）

### 2.1 测试框架配置

```bash
bun add -D vitest @testing-library/svelte jsdom
```

`vite.config.ts` 添加：

```ts
test: {
  environment: 'jsdom',
  include: ['src/**/*.{test,spec}.ts'],
}
```

### 2.2 utils/time.ts

| 函数         | 优先级 | 测试点                              |
| ------------ | ------ | ----------------------------------- |
| `formatMmSs` | P0     | 0秒、59秒、60秒、99分59秒、超过99分 |

### 2.3 utils/date.ts

| 函数              | 优先级 | 测试点                 |
| ----------------- | ------ | ---------------------- |
| `parseYmd`        | P0     | 合法日期、边界月份     |
| `formatYmd`       | P0     | 格式正确性             |
| `todayYmd`        | P1     | 返回当天（mock Date）  |
| `addDays`         | P0     | 正数、负数、跨月、跨年 |
| `startOfWeekYmd`  | P0     | 周一至周日各天         |
| `startOfMonthYmd` | P1     | 各月首日               |
| `endOfMonthYmd`   | P1     | 各月末日、闰年2月      |

### 2.4 utils/phase.ts

| 函数               | 优先级 | 测试点                         |
| ------------------ | ------ | ------------------------------ |
| `phaseLabel`       | P1     | work/shortBreak/longBreak 映射 |
| `phaseAccentClass` | P2     | 各阶段样式类                   |

### 2.5 shared/types.ts

| 测试点                           | 优先级 |
| -------------------------------- | ------ |
| 类型导出正确性（编译时检查即可） | P3     |

### 2.6 api/tauri.ts

| 测试策略                                | 优先级 |
| --------------------------------------- | ------ |
| mock `@tauri-apps/api` 的 invoke/listen | P2     |

### 2.7 stores/_.ts & composables/_.ts

| 测试策略                                  | 优先级 |
| ----------------------------------------- | ------ |
| 状态初始化、更新逻辑（需 mock Tauri API） | P2     |

---

## 三、执行计划

### 阶段一：核心逻辑（预计 2 天）

- [ ] `analysis.rs` 全量测试
- [ ] `timer/notification.rs` 全量测试
- [ ] `utils/time.ts` + `utils/date.ts` 全量测试

### 阶段二：数据层（预计 1 天）

- [ ] `timer/runtime.rs` 补充测试
- [ ] `timer/stats.rs` 补充边界测试
- [ ] `timer/validation.rs` 补充边界测试
- [ ] `app_data.rs` 补充测试

### 阶段三：命令层（预计 2 天）

- [ ] `commands/validation.rs` 测试
- [ ] `commands/settings.rs` 测试
- [ ] `commands/blacklist.rs` 测试
- [ ] 其他 commands 模块

### 阶段四：前端工具层（预计 1 天）

- [ ] 配置 vitest
- [ ] `utils/phase.ts` 测试
- [ ] `api/tauri.ts` mock 测试

### 阶段五：覆盖率检查与补漏（预计 1 天）

- [ ] 运行 `cargo tarpaulin` 检查 Rust 覆盖率
- [ ] 运行 `vitest --coverage` 检查 TS 覆盖率
- [ ] 补充遗漏分支

---

## 四、覆盖率目标分解

| 模块              | 目标覆盖率              |
| ----------------- | ----------------------- |
| `timer/*`         | 95%                     |
| `analysis.rs`     | 90%                     |
| `app_data.rs`     | 90%                     |
| `processes/*`     | 80%（系统调用部分豁免） |
| `commands/*`      | 85%                     |
| `errors.rs`       | 90%                     |
| `src/lib/utils/*` | 95%                     |
| `src/lib/api/*`   | 80%                     |

---

## 五、测试命令

```bash
# Rust 单元测试
cd src-tauri && cargo test

# Rust 覆盖率（需安装 tarpaulin）
cd src-tauri && cargo tarpaulin --out Html

# TypeScript 测试
bun test

# TypeScript 覆盖率
bun test --coverage
```
