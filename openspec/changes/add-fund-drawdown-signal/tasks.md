## 1. 数据源 spike（已完成，见 design.md 决策 1/5）

- [x] 1.1 spike 结论：华夏回报 002001 分红日 `equityReturn` 近 0（派现约占净值 1.9%），证明 `pingzhongdata` 的 `equityReturn` 已复权
- [x] 1.2 数据源改用 `pingzhongdata/<code>.js` 单请求全量（`lsjz` pageSize 实测封顶 20/页不可行）；已回写 spec/design；4 只真实基金黄金基准已算出（见 tasks 8.3）

## 2. 数据模型与 migration

- [x] 2.1 `db/mod.rs` `run_migrations()` 末尾追加 `CREATE TABLE IF NOT EXISTS fund_nav_history(...)`（幂等，portfolio 级联删除）
- [x] 2.2 `models/portfolio.rs` 新增 `FundNavPoint` / `DrawdownWindow` / `FundDrawdownAnalysis`（serde snake_case，复用 performance 的 `DrawdownPoint`），`models/mod.rs` re-export
- [x] 2.3 `src/types/index.ts` 同步 `FundNavPoint`/`DrawdownWindow`/`FundDrawdownAnalysis`/`FundSignalState`

## 3. 净值抓取服务

- [x] 3.1 `services/fund_data.rs` 新增 `fetch_fund_nav_history(fund_code)`，请求 pingzhongdata，复用 `eastmoney_client()`（带 Referer）
- [x] 3.2 正则抽取 `Data_netWorthTrend`（`x/y/equityReturn`）与 `Data_ACWorthTrend`（`[x, acc_nav]`），按时间戳对齐（`acc_map`）
- [x] 3.3 `ts_ms_to_date`（+8h 取 UTC 日期）；返回按日期升序的 `Vec<FundNavPoint>`；解析失败中文错误
- [x] 3.4 单测：解析+排序、时间戳换算、缺 ACWorthTrend 容错、垃圾响应报错（独立 Docker 全绿）

## 4. 复权净值与落库

- [x] 4.1 解析时按 `nav_date` 升序累乘 `equityReturn` 重建 `adjusted_nav`（首点 1.0）；pingzhongdata 单请求全量，无增量衔接问题
- [x] 4.2 `store_fund_nav` 单事务（`unchecked_transaction`）幂等 upsert（`ON CONFLICT(portfolio_id, nav_date) DO UPDATE`），落库才加锁、不跨 `.await`
- [x] 4.3 单测：复权累乘正确（分红 fixture 不向下跳空）、乱序输入按日期重建（独立 Docker 验证）

## 5. 回撤与信号计算

- [x] 5.1 新 `services/fund_drawdown.rs` 以复权净值调用 `calculate_max_drawdown`（`to_points` 适配），产出 HMDD+峰/谷/修复日期+回撤序列
- [x] 5.2 多窗口：全历史 + 近 5 年 + 近 3 年（`window_slice`，字符串 cutoff 自然日回溯）
- [x] 5.3 当前回撤 `CDD`、信号线 `threshold_nav`、`distance_to_signal_pct`
- [x] 5.4 三档信号 `signal_state`（`APPROACHING_RATIO=0.9` 常量；hmdd≈0 货基守卫 → NORMAL）
- [x] 5.5 `history_too_short`（<365 天）；`applicability_note`（按 fund_type：货币/债/指数）
- [x] 5.6 单测：NORMAL/BUY_ZONE/APPROACHING 边界、货基零回撤、历史过短、窗口切片（独立 Docker 全绿）

## 6. IPC command 与注册

- [x] 6.1 `commands/portfolios.rs` 新增 `refresh_fund_nav(portfolioId)`（抓取+`store_fund_nav`，返回分析，中文错误）
- [x] 6.2 新增 `get_fund_drawdown(portfolioId)`（纯读库 `query_fund_nav`+现算）
- [x] 6.3 `lib.rs` `invoke_handler!` 注册两个 command
- [x] 6.4 已确认：启动后台任务仅刷新持仓行情（lib.rs:63-144），未加入净值抓取

## 7. 前端 store 与页面

- [x] 7.1 `portfolioStore.ts` 新增 `refreshFundNav`/`fetchFundDrawdown`、`drawdowns` 缓存、`refreshingNavId`
- [x] 7.2 新增 `src/pages/Funds/DrawdownSignalView.tsx`：信号徽章+关键数字（Statistic）+多窗口表+边界 Alert
- [x] 7.3 复用 `DrawdownChart` 的 ECharts 水下曲线，叠加 HMDD 触发线/接近线/当前线三条 markLine
- [x] 7.4 `PortfolioCard` 加第三个 tab「回撤信号」；头部加信号档位小徽章（挂载时静默读库）
- [x] 7.5 边界 UI：历史过短/适用性提示、离线读库、未抓净值时 Empty 引导先刷新

## 8. 验证与收尾

- [x] 8.1 `npx tsc --noEmit` 通过；后端纯逻辑单测（解析+回撤+信号，11 项）独立 Docker 全绿。本仓 `cargo test` 需开发机/Docker+webkit stub（本机 AL2023 无 webkit）
- [ ] 8.2 端到端手测：开发机对 4 只真实基金刷新净值 → 核对黄金基准（见 manual-test-plan.md）
- [x] 8.3 手测用例模板 `manual-test-plan.md`（含 4 只真实基金黄金基准 + U1~U9 用例）
- [ ] 8.4 隔离性确认：随 8.2 在开发机核对总资产/收益等既有功能数值不变
