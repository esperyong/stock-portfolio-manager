## Why

现有「基金跟踪」只存季度持仓（`portfolios` + `portfolio_positions`），没有任何净值时间序列，因而无法回答一个对定投择时最有用的问题：**这只主动基金现在跌到历史上最惨的程度了吗？** 一个朴素但有效的策略是——当某主动基金的当前回撤达到其历史最大回撤附近时开启定投、回升后停止定投，大概率能买在阶段性大底。本变更补上净值数据与最大回撤信号，把这个策略落成一眼可见的提醒。

## What Changes

- 新增基金**净值历史**抓取与本地存储（天天基金 lsjz 接口，日频），仅用户手动触发，不进入启动后台刷新。
- 基于**复权净值**（由日增长率累乘重建，规避分红除权造成的假回撤）计算：全历史最大回撤（HMDD）、峰值/谷值/修复日期、当前回撤（CDD）、历史最大回撤信号线，并附带**近 3 年 / 近 5 年**对照口径。
- 派生三档**定投信号**（无状态、每次刷新即时计算）：`正常` / `接近历史大底`（CDD ≥ HMDD×0.9）/ `建议开启定投`（CDD ≥ HMDD）；回升到信号线上方即回到 `正常`，对应"停止定投"。
- 「基金跟踪」页面每个基金卡片新增「回撤信号」视图：信号徽章、关键数字、叠加 HMDD 水平线的水下回撤曲线；卡片头部加信号小徽章。

## Capabilities

### New Capabilities
- `fund-drawdown-signal`: 基金净值历史的抓取与存储、基于复权净值的最大回撤/当前回撤计算、定投择时信号（含接近区预警）的派生与展示。

### Modified Capabilities
<!-- 无。本变更为并列的新能力，不改变 fund-tracking 既有需求的行为（持仓展示、调仓对比不变）。 -->

## Impact

- **新增数据表**：`fund_nav_history`（`db/mod.rs` 的 `run_migrations()` 追加 `CREATE TABLE IF NOT EXISTS`，幂等）。
- **后端**：`services/fund_data.rs` 增净值抓取；新增 `services/fund_drawdown.rs`（或复用 `services/performance_service.rs::calculate_max_drawdown`）做回撤计算；`commands/portfolios.rs` 增 `refresh_fund_nav` / `get_fund_drawdown` 两个 command，并在 `lib.rs` 的 `invoke_handler!` 注册；`models/` 增 `FundNavPoint` / `FundDrawdownAnalysis`。
- **前端**：`src/pages/Funds/` 新增 `DrawdownSignalView.tsx` 与卡片信号徽章，复用/改造 `src/pages/Performance/DrawdownChart.tsx`；`src/stores/portfolioStore.ts` 增 `fetchFundDrawdown` / `refreshFundNav`；`src/types/index.ts` 增对应类型。
- **外部依赖**：天天基金 `api.fund.eastmoney.com/f10/lsjz`（需 Referer，复用 `http_client::eastmoney_client()`），非公开接口，与既有 jjcc/搜索接口同类风险。
- **与真实资产域零耦合**：不触及 accounts/holdings/transactions，不进入总资产/收益计算链路。
