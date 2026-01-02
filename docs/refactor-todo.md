# 重构 TODO 清单

## 优先级说明

- P0: 阻塞性问题，必须优先处理
- P1: 高优先级，显著提升可维护性
- P2: 中优先级，改善开发体验
- P3: 低优先级，锦上添花

---

## 后端 (Rust)

### P1: 拆分 `timer.rs` (653行)

- [ ] 创建 `src-tauri/src/timer/` 目录结构
- [ ] 提取 `runtime.rs` - `TimerRuntime` 状态机核心逻辑
- [ ] 提取 `stats.rs` - `TodayStats`/`WeekStats`/`GoalProgress` 统计计算
- [ ] 提取 `notification.rs` - 阶段结束通知、目标达成提醒
- [ ] 提取 `validation.rs` - `validate_settings()` 参数校验
- [ ] 保留 `mod.rs` 作为公开接口，重导出必要类型

### P2: 拆分 `processes.rs` (409行)

- [ ] 提取 `enumeration.rs` - 进程枚举、图标提取
- [ ] 提取 `termination.rs` - 进程终止、权限检测
- [ ] 统一错误处理，移除重复的 `unsafe` 块

### P2: 添加单元测试

- [ ] `timer/stats.rs` - 统计计算测试
- [ ] `timer/runtime.rs` - 状态机转换测试
- [ ] `timer/validation.rs` - 参数校验边界测试
- [ ] `app_data.rs` - 数据迁移测试

---

## 前端 (Svelte)

### P1: 提取业务逻辑 composables

- [ ] 创建 `src/lib/composables/` 目录
- [ ] 提取 `useTimer.ts` - `toggleStartPause`/`resetTimer`/`skipTimer`
- [ ] 提取 `useSettings.ts` - `saveSettings`/`toggleAlwaysOnTop`
- [ ] 提取 `useBlacklist.ts` - `saveBlacklist`/模板管理
- [ ] 提取 `useTags.ts` - `onTagSelectChange`/`addAndSelectTag`
- [ ] 提取 `useToast.ts` - toast 状态管理

### P2: 简化 `+page.svelte`

- [ ] 移除内联业务逻辑，改为调用 composables
- [ ] 拆分为更小的子组件（如 `TimerCard.svelte`、`StatsCard.svelte`）
- [ ] 统一事件处理命名规范

### P3: 组件优化

- [ ] `BlacklistModal.svelte` - 拆分进程列表与模板选择
- [ ] `SettingsModal.svelte` - 按分组拆分表单区块

---

## 工程化

### P1: 类型同步

- [ ] 评估 `ts-rs` crate 自动生成 TypeScript 类型
- [ ] 或添加 CI 检查确保 `types.ts` 与 Rust 结构体一致

### P2: 代码质量

- [ ] 配置 `cargo clippy` 更严格的 lint 规则
- [ ] 前端添加 ESLint + Prettier 统一风格
- [ ] 添加 pre-commit hooks

---

## 实施顺序建议

1. **第一阶段**: 拆分 `timer.rs` + 添加核心测试
2. **第二阶段**: 提取前端 composables + 简化页面
3. **第三阶段**: 类型自动生成 + 工程化配置

---

## 注意事项

- `timer.rs` 拆分时需确保 `tick()` 方法的原子性不被破坏
- 前端 composables 需兼容 Svelte 5 runes 响应式系统
- 重构过程中保持 `bun run check` 和 `cargo clippy` 通过
