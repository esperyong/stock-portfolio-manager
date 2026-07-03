# 提案：新增「基金跟踪」能力（v1）

## Why

用户长期跟踪若干主动型公募基金（如兴全系），希望从基金经理的真实调仓行为中提取选股信号。但基金全量持仓每半年才披露一次（年报/中报，滞后 2~3 个月），日常只能依赖季报的前十大重仓（滞后约 15 个工作日）。目前这些数据靠手工下载 PDF、人工提取，效率低且无法沉淀为结构化数据。

天天基金/东方财富的接口已实测验证：可搜索基金、可直接取回结构化的各期持仓明细（含**持股数**——后续信号分析的关键原料，权重口径会被股价涨跌污染）。本次变更把「关注基金 + 查看最新前十大持仓」做进应用，并从第一天起将持仓快照落库，为后续的调仓信号与抱团趋势分析（不在本次范围）打好数据地基。

## What Changes

- 新增「基金跟踪」页面：管理关注基金列表（添加/删除），查看每只基金最新一期的前十大持仓。
- 添加基金时通过天天基金 `fundsuggest` 接口做名称/代码联想搜索，用户从候选中选择。
- 通过东财 `FundArchivesDatas.aspx (type=jjcc)` 接口拉取持仓明细，展示：报告期截止日期、股票代码/名称、占净值比例、持股数（万股）、持仓市值（万元）。
- 持仓快照持久化到 SQLite（`watched_funds` + `fund_holding_snapshots` 两张新表，幂等 upsert），展示优先读库、手动刷新时更新。
- 刷新为**手动按钮触发**，不加入启动后台刷新任务（季度级数据，无需实时）。
- 后端新增 `services/fund_data.rs` 与 `commands/funds.rs`，命令在 `lib.rs` `invoke_handler!` 注册；错误信息中文。
- 前端新增 `fundStore` 与 `Funds` 页面，TS 类型与 Rust struct 手工镜像（snake_case）。

明确不做（后续阶段）：调仓 diff 与信号计算、跨基金抱团趋势聚合、markdown 报告导出、基金净值/估值展示、个股反查十大流通股东。

## Capabilities

### New Capabilities

- `fund-tracking`: 关注基金的管理（搜索、添加、删除、列表）与最新前十大持仓的获取、落库、展示。

### Modified Capabilities

（无——本仓尚无已归档的主 spec，且本次不改动现有股票组合管理行为。）

## Impact

- **后端**：新增 `src-tauri/src/services/fund_data.rs`（东财 HTTP 接口调用 + jjcc HTML 表解析）、`src-tauri/src/commands/funds.rs`（IPC 命令）、`src-tauri/src/models/` 新增基金相关 struct；`lib.rs` 注册命令；`db/mod.rs` 的 `run_migrations()` 末尾追加两张新表。
- **前端**：新增 `src/stores/fundStore.ts`、`src/pages/Funds/`（页面 + 组件）、`src/types/index.ts` 新增镜像类型；导航新增入口。
- **依赖**：复用现有 `reqwest`/`tokio`（行情服务已在用），HTML 解析优先用正则或轻量方式，避免引入重依赖。
- **外部风险**：数据源为东财非公开接口（需 `Referer` 头），可能变更或限流——快照落库天然提供降级（接口失败时仍展示上次数据 + 中文警告），与现有行情缓存的降级思路一致。
- **数据库**：仅新增表，无对既有表的修改，migration 保持幂等可追加。
