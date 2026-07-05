# 提案：新增「基金跟踪」能力（v1）——组合 + 仓位模型

## Why

用户长期跟踪若干主动型公募基金（如兴全系），希望从基金经理的真实调仓行为中提取选股信号。但基金全量持仓每半年才披露一次（年报/中报，滞后 2~3 个月），日常只能依赖季报的前十大重仓（滞后约 15 个工作日）。目前这些数据靠手工下载 PDF、人工提取，效率低且无法沉淀为结构化数据。

天天基金/东方财富的接口已实测验证：可搜索基金、可直接取回结构化的各期持仓明细（含**持股数**——后续信号分析的关键原料，权重口径会被股价涨跌污染）。本次变更把「关注基金 + 查看最新前十大持仓」做进应用，并引入**雪球式的「组合 + 仓位」模型**（组合 = 权重配比的版本序列，无成本概念）作为数据地基——基金披露的每一期持仓即组合的一个版本，为后续的调仓 diff、信号计算与抱团趋势分析（不在本次范围）铺路。

## What Changes

- 新增通用数据模型：`portfolios`（组合，来源分 `FUND`/`MANUAL`）+ `portfolio_positions`（仓位，按 `as_of_date` 版本化）。**权重为一等公民，无成本字段**；股数/市值为基金来源的附加事实。v1 仅实现 `FUND` 来源，`MANUAL`（用户手工组合）只留 schema，不做 UI。
- 新增「基金跟踪」页面：从天天基金联想搜索中选择基金创建组合（添加/删除），查看每个基金组合最新一期的前十大仓位。
- 通过东财 `FundArchivesDatas.aspx (type=jjcc)` 接口拉取持仓明细，展示：报告期截止日期、股票代码/名称、占净值比例、持股数（万股）、持仓市值（万元）。
- 持仓按报告期幂等落库（`UNIQUE(portfolio_id, as_of_date, stock_code)`），展示优先读库，离线可用。
- 刷新为**手动按钮触发**，不加入启动后台刷新任务（季度级数据，无需实时）。
- 后端新增 `services/fund_data.rs` 与 `commands/portfolios.rs`，命令在 `lib.rs` `invoke_handler!` 注册；错误信息中文。
- 前端新增 `portfolioStore` 与 `Funds` 页面（路由 `/funds`），TS 类型与 Rust struct 手工镜像（snake_case）。

明确不做（后续阶段，各自另立 change）：调仓 diff 与信号计算、跨基金抱团趋势聚合、**主动偏离度**（基金权重 − 指数权重，以 ETF 全量成分为基准，已验证同一 jjcc 通路可取指数权重）、**ETF 每日 PCF 申购赎回清单接入**（日频全量成分+精确股数，数据源待 spike 验证）、组合净值化收益曲线（雪球式模拟收益）、MANUAL 组合的创建/编辑 UI、markdown 报告导出、基金净值/估值展示、个股反查十大流通股东。

## Capabilities

### New Capabilities

- `fund-tracking`: 以「组合 + 仓位」模型承载的基金跟踪：基金搜索、基金组合的创建/删除/列表，各报告期仓位的抓取、版本化落库与最新前十大展示。

### Modified Capabilities

（无——本仓尚无已归档的主 spec，且本次不改动现有股票组合管理行为。）

## Impact

- **后端**：新增 `src-tauri/src/services/fund_data.rs`（东财 HTTP 接口调用 + jjcc HTML 表解析）、`src-tauri/src/commands/portfolios.rs`（IPC 命令）、`src-tauri/src/models/` 新增 `Portfolio`/`PortfolioPosition`/`FundSearchResult`；`lib.rs` 注册命令；`db/mod.rs` 的 `run_migrations()` 末尾追加两张新表。
- **前端**：新增 `src/stores/portfolioStore.ts`、`src/pages/Funds/`、`src/types/index.ts` 新增镜像类型；导航新增「基金跟踪」入口。
- **与现有模型的关系**：与 `accounts`/`holdings`/`transactions`（真实资产域）**零外键耦合**——组合是权重配比，不是真实持仓，不进入市值合计/收益计算/快照等下游链路。共享的是基础设施：DB mutex、migration 追加风格、命令分层、行情源（后续净值化阶段复用）。
- **依赖**：复用现有 `reqwest`/`tokio`，HTML 解析用正则，不引入新依赖。
- **外部风险**：数据源为东财非公开接口（需 `Referer` 头），可能变更或限流——仓位落库天然提供降级（接口失败时仍展示上次数据 + 中文警告）；基金历史可随时从接口回补（回溯至 2013）。
- **数据库**：仅新增表，无对既有表的修改，migration 保持幂等可追加。
