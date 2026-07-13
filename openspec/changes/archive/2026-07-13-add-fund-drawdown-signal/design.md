## Context

「基金跟踪」现有能力只覆盖季度持仓（`portfolios` + `portfolio_positions`，见 `openspec/specs/fund-tracking`），没有净值时间序列。最大回撤是净值序列上的统计量，因此本变更必须先补齐**基金净值抓取与存储**这一新数据轴，再在其上计算回撤与信号。

有利的现状：
- `services/performance_service.rs::calculate_max_drawdown` 已为**真实组合**（`daily_portfolio_values`）实现了最大回撤 + 峰值/谷值/修复日期 + 回撤序列；`models/performance.rs` 有 `DrawdownAnalysis`/`DrawdownPoint`；前端 `src/pages/Performance/DrawdownChart.tsx` 已画好水下曲线。算法与 UI 大量可复用，主要工作是「把输入从真实组合日值换成基金复权净值」并叠加信号语义。
- 基金域与真实资产域零外键耦合（`commands/portfolios.rs` 头部注释），新数据表沿用此隔离。
- 数据源 `api.fund.eastmoney.com/f10/lsjz` 已实测可用（005827 返回 1879 条、110011 返回 4353 条），字段 `FSRQ/DWJZ/LJJZ/JZZZL/TotalCount` 齐全。

约束：Tauri IPC command 双处注册（`commands/` + `lib.rs` 的 `invoke_handler!`）；Rust struct 与 TS interface 手工同步，serde 用 snake_case 原样字段名；migration 只追加、幂等；DB 单 `Mutex` 串行，锁段短、勿跨 `.await` 持锁；错误串中文。

## Goals / Non-Goals

**Goals:**
- 抓取并本地存储基金日频净值（单位/累计/日增长率），增量刷新，手动触发。
- 以复权净值为基准计算全历史 + 近 3 年/近 5 年最大回撤、当前回撤、信号线。
- 派生无状态三档定投信号（正常 / 接近历史大底 / 建议开启定投）并在 UI 呈现（徽章 + 关键数字 + 叠加 HMDD 线的水下曲线）。
- 复用既有回撤算法与图表组件，最小化新代码面。

**Non-Goals:**
- 不做持久化「定投窗口」状态机（v1 无状态即时派生）。
- 不做自动定投下单、金额/份额计算，不与 accounts/holdings/transactions 耦合。
- 不做盘中估值（`fundgz`）实时信号；只用日频落库净值。
- 不进入应用启动的后台自动刷新任务。
- 不做跨基金聚合/抱团（属 fund-tracking 的 v3 路线）。

## Decisions

### 决策 1：回撤基准用「复权净值」，由复权日收益率累乘重建（spike 已确认）
- **选择**：`adjusted_nav(0)=1.0`，`adjusted_nav(t)=adjusted_nav(t−1)×(1 + equityReturn(t)/100)`，回撤全程基于该序列。`equityReturn` 取自 `pingzhongdata` 的 `Data_netWorthTrend`。
- **理由**：单位净值在分红除权日下跌 → 假回撤（110011 实测单位净值 3.99 vs 累计净值 5.79，分红已使两者严重分叉）。累计净值虽加回分红但分红部分是"死数"，当累计分红占比大时会**低估**真实回撤。`equityReturn` 是分红再投资口径的真实日收益，累乘即等价复权净值。
- **备选**：直接用累计净值——实现更省一步，但对高分红老基金系统性低估回撤，与"找历史大底"目标冲突，故仅作备查字段。
- **spike 结论（已实测，2026-07-13）**：以频繁分红的华夏回报 002001 验证——分红点 `2003-12-15` 每份派现 0.0198 元（约占净值 1.9%），但当日 `equityReturn=-0.019%`（近 0），**证明 `equityReturn` 已复权、剔除了除权下跌**。`Data_netWorthTrend` 共 5542 点、112 个分红/拆分点带 `unitMoney` 标记。首点 `equityReturn=0`、基准 1.0。无缺失段，无需回退。

### 决策 2：新增独立表 `fund_nav_history`，portfolio 级联删除，回撤读时现算
- **选择**：
  ```sql
  CREATE TABLE IF NOT EXISTS fund_nav_history (
      portfolio_id TEXT NOT NULL REFERENCES portfolios(id) ON DELETE CASCADE,
      nav_date     TEXT NOT NULL,   -- YYYY-MM-DD (FSRQ)
      unit_nav     REAL,            -- 单位净值 DWJZ
      acc_nav      REAL,            -- 累计净值 LJJZ
      adjusted_nav REAL,            -- 复权净值（重建，回撤基准）
      daily_return REAL,            -- 日增长率 JZZZL(%)
      PRIMARY KEY (portfolio_id, nav_date)
  );
  ```
- **理由**：沿用 `portfolio_positions` 的 `portfolio_id + ON DELETE CASCADE` 模式，保持基金域隔离与"删组合真删、可回补"的既有哲学。HMDD 在 ~4000 点上计算 <1ms，**不落库、读时现算**，避免引入需要维护一致性的派生列（与 options 把 `contract_status` 落库那种"计算昂贵才缓存"的场景不同）。
- **备选**：以 `fund_code` 为键、跨组合生命周期独立存储，可省去重加基金后的重抓（~4000 行）。因 `portfolios.fund_code` UNIQUE、组合与基金 1:1，收益有限，且破坏级联真删的一致性，故不采用。

### 决策 3：复用 `calculate_max_drawdown`，泛化输入 + 新增信号层
- **选择**：把回撤核心算法抽成接受 `&[(date, value)]` 的通用函数（供真实组合与基金共用），或在新 `services/fund_drawdown.rs` 内以复权净值序列调用/重写等价逻辑；信号派生（三档 + 信号线 + 距离 + 多窗口）单独成层。返回模型 `FundDrawdownAnalysis` 扩展 `DrawdownAnalysis` 的字段（增 `current_drawdown / threshold_nav / distance_to_signal_pct / signal_state / windowed{3y,5y} / history_too_short`）。
- **理由**：算法已验证正确，避免重复实现回撤/峰谷/修复逻辑；信号是薄薄一层纯计算，独立可测。
- **备选**：完全独立实现一套 —— 重复代码、双份 bug 面，不采用。

### 决策 4：信号无状态即时派生，接近区系数 0.9（常量）
- **选择**：命令每次读取时用最新净值现算三档状态；接近区阈值 `HMDD×0.9` 以后端常量表达，便于日后调参。
- **理由**：用户已确认 v1 无状态；开启/停止循环由"每次刷新重算布尔"天然覆盖，无需事件表。系数硬编码常量足够，暂不引入配置项（避免 `quote_provider_config` 式的设置面扩张）。

### 决策 5：数据源用 `pingzhongdata` 单请求全量，放弃 lsjz 分页
- **选择**：`fetch_fund_nav_history(fund_code)` 请求 `fund.eastmoney.com/pingzhongdata/<code>.js`（带 Referer），正则抽取 `Data_netWorthTrend`（`x/y/equityReturn/unitMoney`）与 `Data_ACWorthTrend`（`[x, acc_nav]`）两个 JSON 数组，按时间戳 `x` 对齐。`x` 为东八区零点的 UTC 毫秒，换算 `nav_date` 时须 +8h 再取 UTC 日期。全序列按日期升序累乘 `equityReturn` 重建 `adjusted_nav`（首点 1.0），一次性幂等 upsert。
- **理由**：`lsjz` 的 pageSize 实测**封顶 20 行/页**（≥499 直接返回 null），全历史 5000+ 行需数百次请求，既慢又触发限流；`pingzhongdata` **一次请求即含全部历史**（002001 实测 714KB、5542 点），且自带复权收益率 `equityReturn` 与分红标记 `unitMoney`，正是所需。
- **权衡**：`pingzhongdata` 单文件 ~700KB 且含大量无关数据（仓位测算、规模、经理等），每次刷新都全量传输——但净值手动刷新、跟踪基金数量少，可接受。因每次都拿全量历史，`adjusted_nav` 直接对全序列重建即可，无需"增量衔接已存末值"的复杂度。
- **备选**：保留 `lsjz` 做增量补新——因单请求全量已足够简单且省心，不引入第二数据源。

## Risks / Trade-offs

- **[非公开接口/限流] `lsjz` 需 Referer、可能变更或限流** → 复用 `http_client::eastmoney_client()` 的浏览器头与超时；失败仅置中文错误、不动已落库数据；与既有 jjcc/搜索同类风险，可接受。
- **[复权口径存疑] 日增长率是否全程已复权尚需实测** → 上线前做决策 1 的 spike；回退方案为 `LJJZ` 近似并标注。
- **[老基金信号永不触发] 全历史 HMDD 可能来自极早期极端行情** → 同屏展示近 3 年/近 5 年对照口径，让用户判断触发现实性；主信号仍按用户明确要求用全历史。
- **[历史过短误导] 新基金 HMDD 不稳** → 不足一年标记「参考意义有限」。
- **[适用性] 策略针对主动权益基金** → 对所有基金都算不硬过滤（货基 MDD≈0 无害），按 `fund_type` 给适用性提示。
- **[增量复权衔接错误] 若新段未接续已存 `adjusted_nav` 末值会产生台阶** → 落库逻辑显式以已存末日复权值为递推起点，并加单测覆盖"增量拼接后序列连续"。
- **[净值 T+1 与持仓刷新耦合] 一个刷新按钮 vs 两个** → 净值数据量远大于持仓，倾向独立的净值刷新入口/独立 command，避免拖慢持仓刷新；具体按钮编排在 tasks 中定。

## Open Questions

- ~~分红日复权口径 spike~~ —— 已确认（见决策 1），`equityReturn` 已复权，无需回退。
- 净值刷新入口：并入现有「刷新」按钮，还是「回撤信号」视图内独立「刷新净值」按钮？（倾向后者，最终在实现时确认交互）
- 近 3 年/近 5 年窗口的边界取法（按自然日回溯 vs 按交易日回溯）——实现时统一为按自然日回溯到最近可得净值。
