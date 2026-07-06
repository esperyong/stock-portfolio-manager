# 设计：基金跟踪 v2——调仓 diff

## Context

v1 已落库 `portfolio_positions`（`UNIQUE(portfolio_id, as_of_date, stock_code)`，含 `weight_pct` / `shares_wan` / `market_value_wan` / `position_rank`），主 spec 基线在 `openspec/specs/fund-tracking/spec.md`。数据的领域特性（v1 实测确认）：

- 季报期（Q1/Q3，以及中报/年报发布前的 Q2/Q4）只披露**前十大 + `*` 号行**（实测 10~26 行，指数基金约 15 行）；中报/年报发布后 Q2/Q4 版本被**全量持仓**覆盖（实测 60~120 行）。
- 同一 `as_of_date` 的数据会随披露进程**变富**（前十大 → 全量），v1 的幂等 upsert 已保证只增不重。
- 股数（万股）是信号分析的关键口径；权重口径被股价涨跌污染（v1 提案已论证）。

本变更纯读库计算，不触网、不改 schema。

## Goals / Non-Goals

**Goals：**
- 任意组合的两个版本间逐股 diff：新建/清仓/加仓/减仓/持平，股数主口径、权重辅助。
- 披露口径感知：跨口径对比时不产生错误的"清仓/新建仓"断言。
- 版本列表查询（供 UI 期次选择器），默认对比最新两期。
- diff 逻辑为纯函数、全量单测覆盖。

**Non-Goals（后续另立 change）：**
- 调仓信号阈值/提醒、跨基金抱团聚合、主动偏离度、PCF 接入。
- 送转股复权（见 Risks）、diff 结果导出、MANUAL 组合的 diff UI 优化。

## Decisions

### D1. 股数主口径 + epsilon 持平判定，权重仅作辅助展示
分类依据 `shares_wan` 差值：`|Δshares| ≤ 0.001` 视为持平（上游数据精确到 0.01 万股）。仅当某侧 `shares_wan` 缺失（老数据 `---`）时回退权重口径判定加减，并在结果中标记 `basis: "weight"`。`Δweight_pct`（百分点）与 `Δmarket_value_wan` 一律附带返回，仅作展示不参与分类。

### D2. 披露口径推断：查询时启发式，不加列
```
coverage(as_of_date, row_count):
    月份 ∈ {03, 09}          → PARTIAL   （只有季报，恒为前十大口径）
    月份 ∈ {06, 12}:
        row_count ≥ 35       → FULL      （中报/年报全量已落库）
        否则                  → PARTIAL   （尚处季报前十大阶段）
```
不新增 DB 列（v1 设计预案：需要时再加，migration 可追加）。误判方向是保守的：持仓极度集中的基金全量 <35 行会被判为 PARTIAL，后果只是标签措辞更谨慎，不会虚构"清仓"。阈值 35 依据实测：部分披露最多 ~26 行，全量最少 ~60 行，中间地带宽。

### D3. Diff 语义矩阵：change_type 保持中立，标签按**方向**由口径决定
后端 `change_type` 只描述**披露名单层面的事实**：`NEW`（此侧出现）/ `EXITED`（此侧消失）/ `INCREASED` / `DECREASED` / `UNCHANGED`。响应同时携带 `from_coverage` / `to_coverage`，**由前端按方向渲染措辞**——出现的确定性取决于**起始侧**口径（起始为全量，才能断言此前未持有），消失的确定性取决于**目标侧**口径（目标为全量，即可断言现已不持有）：

| 判定 | 条件 | 渲染为 |
|---|---|---|
| NEW | from = FULL | 新建仓（确定） |
| NEW | from = PARTIAL | 新进披露（此前可能已持有） |
| EXITED | to = FULL | 清仓（确定） |
| EXITED | to = PARTIAL | 退出披露（可能仍持有） |

实测校验（163417，2025-09-30 部分 → 2025-12-31 全量）：中芯国际从 12-31 全量名单消失 = 可确证的真清仓——按方向判定能保留这一信息，"任一侧部分即全部保守"会丢失它。

关键不变量：**两侧共同出现的股票，其股数对比不受口径影响**（部分披露中的股数是真实值），加减仓判断始终有效。

### D4. 命令面（`commands/portfolios.rs` 追加，读库即算，不持锁跨 await——本变更无 await）
| 命令 | 参数(camelCase) | 行为 |
|---|---|---|
| `get_portfolio_versions` | `portfolioId` | 版本列表：`as_of_date` 降序，含 `row_count`、`coverage` |
| `get_portfolio_diff` | `portfolioId, fromDate?, toDate?` | 省略时取最新两期；版本不足两期报中文错误「该组合尚不足两期持仓数据，无法对比」 |

diff 响应结构：`{ from_version, to_version, items }`，`items` 内按类别排序（NEW → INCREASED → DECREASED → EXITED → UNCHANGED），类别内按 `|Δweight_pp|` 降序。DiffItem 字段：`stock_code, stock_name, change_type, basis, from_shares_wan, to_shares_wan, shares_delta_wan, shares_delta_pct, from_weight_pct, to_weight_pct, weight_delta_pp, to_market_value_wan, from_rank, to_rank`。

### D5. 纯函数服务层：`services/position_diff.rs`
`infer_coverage(as_of_date, row_count) -> Coverage` 与 `compute_diff(from_rows, to_rows) -> Vec<DiffItem>` 均为无 IO 纯函数；命令层只负责 SQL 取数与参数校验。单测直接喂内存数据，不依赖 DB 与网络。

### D6. 前端：卡片展开区改双视图
`src/pages/Funds/` 卡片展开区改为两个页签：「最新持仓」（v1 原表）/「调仓」（新增 `PositionDiffView` 组件）。调仓视图：两个期次下拉（默认最新两期）→ 分类明细表（变动类型 Tag、股数 from→to 及 ±%、权重变化 pp、最新市值）；涉及 PARTIAL 版本时顶部显示提示条「该期仅披露前十大重仓，『退出披露』不代表清仓」。`portfolioStore` 增 `fetchVersions` / `fetchDiff`；`types/index.ts` 镜像 `PortfolioVersion` / `PositionDiff` / `PositionDiffItem`（snake_case）。

## Risks / Trade-offs

- [送转/拆股污染股数口径] A 股送转（如 10送10）会使股数翻倍而非真实加仓 → v2 只陈述事实（"持股数 +100%"）不做信号断言；复权处理留给信号计算 change（届时可复用 `stock_splits` 表的思路）。
- [口径启发式误判] 集中持仓基金全量 <35 行 → 误判为 PARTIAL，标签偏保守（可接受）；若未来发现反例，可改为落库时记录来源表行数或加列。
- [同日期版本数据变富] Q2 季报（前十）落库后中报（全量）覆盖同一 `as_of_date` → diff 的"版本"始终是当前库内该日期的最新形态，行为自然正确，无需特殊处理。
- [两侧股数均缺失] 极老数据双侧 `---` → 回退权重口径（`basis: "weight"`），UI 注明"按权重估算"。

## Migration Plan

无 DB 变更。纯增量：新增服务文件 + 两个命令 + 前端组件，回滚即删除。版本号随发布正常递增。

## Open Questions

（无阻塞项。送转复权、信号阈值已明确移入后续 change。）
