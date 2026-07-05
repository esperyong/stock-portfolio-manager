# 设计：基金跟踪 v1（组合 + 仓位模型）

## Context

应用现为「账户/持仓/交易」三层的**真实资产**管理器（Tauri 2.0：Rust 后端 + React 前端 + SQLite）。本次新增与之平行的「组合」域：参照雪球模拟组合的抽象——**组合是权重配比的版本序列，没有成本概念**。v1 的组合来源是公募基金的定期披露（每期披露 = 组合的一个版本），后续可扩展用户手工组合与净值化收益。数据源为东财/天天基金非公开接口，均已实测验证（2026-07）：

- **搜索**：`https://fundsuggest.eastmoney.com/FundSearch/api/FundSearchAPI.ashx?m=1&key=<关键词>` — 名称/代码/拼音联想，返回 JSON（代码、名称、类型）。
- **持仓明细**：`https://fundf10.eastmoney.com/FundArchivesDatas.aspx?type=jjcc&code=<码>&topline=200&year=<年>` — 返回 `var apidata={content:"<按季度的HTML表>", arryear:[可查年份], curyear:...}`。季报表固定 10 行（前十大）；Q2/Q4 表在中报/年报发布后为**全量持仓**（实测兴全模式 2025：Q4 表 69 行、Q2 表 65 行）。每行含：排名、股票代码、名称、占净值比例(%)、持股数（万股）、持仓市值（万元）。历史可回溯至 2013。
- 两者都需要 `Referer` 头（`https://fundf10.eastmoney.com/` 一类），否则拒绝。

## Goals / Non-Goals

**Goals：**
- 建立通用的「组合 + 仓位」数据模型（权重为一等公民、按日期版本化、无成本）。
- 用户在 UI 中搜索基金并创建基金组合、删除组合。
- 每个基金组合可查看**最新一期**的前十大仓位（含截止日期、权重、股数、市值）。
- 拉取到的各期仓位幂等落库，展示优先读库；接口失败时仍能展示上次数据。
- 手动刷新，不进启动后台刷新任务。

**Non-Goals（后续阶段）：**
- 调仓 diff（相邻版本对比）、股数口径信号计算、跨基金抱团趋势聚合。
- 组合净值化收益曲线（权重 × 行情的雪球式模拟收益）。
- MANUAL 组合的创建/编辑 UI（schema 就位，界面后续）。
- markdown 报告导出、基金净值/估值展示、个股侧十大流通股东反查。

## Decisions

### D1. 组合模型：一只基金 = 一个 portfolio，仓位按 `as_of_date` 版本化
参照雪球模拟组合：组合的本质是**目标权重配比**；"调仓"即产生新版本。基金域里天然映射为：每个披露报告期 = 一个版本。不采用"每期一个 portfolio"（组合列表会被季度数据刷爆，跨期 diff 也别扭）。

```sql
CREATE TABLE IF NOT EXISTS portfolios (
    id TEXT PRIMARY KEY NOT NULL,            -- uuid，与 accounts/holdings 风格一致
    name TEXT NOT NULL,                      -- 基金完整名称，或（后续）用户自定义名
    source_type TEXT NOT NULL CHECK(source_type IN ('FUND','MANUAL')),
    fund_code TEXT UNIQUE,                   -- source_type='FUND' 时非空；MANUAL 为 NULL
    fund_type TEXT,                          -- 混合型-偏股 等，来自搜索接口
    last_refreshed_at TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS portfolio_positions (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    portfolio_id TEXT NOT NULL REFERENCES portfolios(id) ON DELETE CASCADE,
    as_of_date TEXT NOT NULL,                -- 基金=报告期截止日（YYYY-MM-DD）；MANUAL=生效日
    stock_code TEXT NOT NULL,                -- A股6位 / 港股5位（带前导零原样存）
    stock_name TEXT NOT NULL,
    weight_pct REAL,                         -- 占净值比例(%)，一等公民
    shares_wan REAL,                         -- 持股数（万股）——FUND 来源附加事实，信号分析关键字段
    market_value_wan REAL,                   -- 持仓市值（万元）——FUND 来源附加事实
    position_rank INTEGER,                   -- 表内排名
    created_at TEXT NOT NULL,
    UNIQUE(portfolio_id, as_of_date, stock_code)
);
```

**无成本字段**：组合域刻意不含 `avg_cost`/交易/现金概念，与真实资产域（`holdings` 由 transactions 反演重放维护成本）划清边界，零外键耦合。后续净值化收益直接由权重 × 行情计算。

### D2. 删除 = 级联真删
`ON DELETE CASCADE` 连仓位历史一起删。基金历史可随时从 jjcc 接口回补（回溯至 2013），删了不丢东西；MANUAL 组合不可回补，届时前端加确认对话框（v1 无 MANUAL UI，不涉及）。相比"软删/归档"少一个状态位、少一类边界。

### D3. jjcc HTML 用正则解析，不引入 HTML 解析依赖
表格行模式固定（`<tr><td>序号</td>…<a>代码</a>…<a>名称</a>…<td class='tor'>数值</td>…`），已用同模式的正则在真实数据上验证。解析失败（上游改版）时返回中文错误，不影响已落库数据。
**注意**：股票代码按 `\d{5,6}` 匹配——基金可持港股通标的（5 位代码，如 00700），只匹配 6 位会漏。港股代码带前导零**原样存**；本仓真实资产域的 HK symbol 约定是去前导零（见 `tools/normalize_hk_symbols`），后续任何把组合仓位 join 到行情/holdings 的功能需做代码归一（去前导零 + market 推断），v1 只展示不关联行情，无此问题。

### D4. 单次抓取解析当年**全部**期次并全部落库，展示只取最新一期
jjcc 按年返回该年所有已披露期次（1 个 HTTP 请求）。全部 upsert 成本为零，却为后续「调仓 diff」免费积累版本。若当年尚无任何期次（年初 1~4 月季报未出），自动回退拉上一年。`as_of_date` 由表标题推断（Q1=03-31、Q2=06-30、Q3=09-30、Q4=12-31）。「最新一期」= 各版本中 `as_of_date` 最大者；年报/中报全量表也只展示前 10 行（表本身按权重降序）。

### D5. IPC 命令面（`commands/portfolios.rs`，注册进 `lib.rs` `invoke_handler!`）
| 命令 | 参数(camelCase) | 行为 |
|---|---|---|
| `search_funds` | `keyword` | 调 fundsuggest，返回候选列表（不落库） |
| `create_fund_portfolio` | `fundCode, fundName, fundType` | 建 `source_type='FUND'` 组合（fund_code 重复报中文错误），随后自动做一次首刷 |
| `delete_portfolio` | `portfolioId` | 级联删除组合与全部仓位版本 |
| `list_portfolios` | — | 组合列表 + 每个组合最新 `as_of_date` |
| `refresh_fund_portfolio` | `portfolioId` | 抓取→解析→upsert→更新 `last_refreshed_at`→返回最新一期仓位 |
| `get_portfolio_positions` | `portfolioId` | 只读库：最新一期全部行（前端截前 10 展示） |

网络请求为 async（复用 reqwest/tokio）；**先完成 HTTP 与解析，再 `db.conn.lock()` 做 upsert**，绝不跨 `.await` 持锁（遵守 CLAUDE.md）。错误 `Result<T, String>` 中文文案。

### D6. 前端：`portfolioStore` + `Funds` 页，路由 `/funds`
- `src/stores/portfolioStore.ts`：照现有 store 形状（loading / error / invoke 封装），组件只消费 store。
- `src/pages/Funds/`：基金组合卡片列表（名称/代码/类型/最新报告期/上次刷新时间）；「添加基金」弹窗内做防抖搜索下拉；卡片展开显示仓位表（截止日期 + 前十大：代码/名称/权重/**股数**/市值）+ 每卡「刷新」「删除」按钮。
- `App.tsx` 加 `<Route path="/funds">`，侧边导航加「基金跟踪」入口。
- `src/types/index.ts` 新增 `Portfolio` / `PortfolioPosition` / `FundSearchResult`，字段 snake_case 与 Rust struct 一致。

### D7. 多份额（A/C）不做归一
同一基金的 A/C 份额代码不同但持仓相同。v1 由用户自行选择添加哪个份额，不做份额合并/去重——搜索结果中已显示完整名称（含 A/C 后缀），用户可辨别。

## Risks / Trade-offs

- [非公开接口变更/限流] → 仓位落库天然降级：刷新失败时展示上次数据 + 中文警告；解析用防御式正则，失败即报错不写脏数据。频率本身极低（手动、季度级）。
- [HTML 表结构改版] → 解析逻辑集中在 `fund_data.rs` 单一函数，配单元测试（用真实响应片段做 fixture），改版时修一处。
- [报告期语义粗化] v1 用「表标题→季度末日期」推断 `as_of_date`，未区分数据出自季报还是年报全量（Q4 表两种来源同日期）。对"最新前十大"展示无影响；后续做 diff 时如需区分来源，可加列（migration 追加即可）。
- [MANUAL 分支的 schema 先行] `source_type='MANUAL'` 仅建约束不建功能，存在"死 schema"风险 → 代价是一个 CHECK 枚举值，远小于日后迁移表名/加来源列的成本。

## Migration Plan

仅追加两张 `CREATE TABLE IF NOT EXISTS`，无既有表改动，天然幂等可回滚（新表不被旧代码引用）。版本号 `package.json` 与 `Cargo.toml` 同步递增。

## Open Questions

（无阻塞项。信号阈值、净值化收益、MANUAL 组合 UI、报告导出等已明确移入后续阶段。）
