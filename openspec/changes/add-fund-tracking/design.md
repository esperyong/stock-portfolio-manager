# 设计：基金跟踪 v1

## Context

应用现为「账户/持仓/交易」三层的股票组合管理器（Tauri 2.0：Rust 后端 + React 前端 + SQLite）。本次新增与股票组合**平行**的「基金跟踪」域：关注基金列表 + 最新前十大持仓展示，快照落库。数据源为东财/天天基金非公开接口，均已实测验证（2026-07）：

- **搜索**：`https://fundsuggest.eastmoney.com/FundSearch/api/FundSearchAPI.ashx?m=1&key=<关键词>` — 名称/代码/拼音联想，返回 JSON（代码、名称、类型）。
- **持仓明细**：`https://fundf10.eastmoney.com/FundArchivesDatas.aspx?type=jjcc&code=<码>&topline=200&year=<年>` — 返回 `var apidata={content:"<按季度的HTML表>", arryear:[可查年份], curyear:...}`。季报表固定 10 行（前十大）；Q2/Q4 表在中报/年报发布后为**全量持仓**（实测兴全模式 2025：Q4 表 69 行、Q2 表 65 行）。每行含：排名、股票代码、名称、占净值比例(%)、持股数（万股）、持仓市值（万元）。历史可回溯至 2013。
- 两者都需要 `Referer` 头（`https://fundf10.eastmoney.com/` 一类），否则拒绝。

## Goals / Non-Goals

**Goals：**
- 用户在 UI 中搜索并添加/删除关注基金。
- 每只基金可查看**最新一期**的前十大持仓（含截止日期、权重、股数、市值）。
- 拉取到的持仓按报告期幂等落库，展示优先读库；接口失败时仍能展示上次数据。
- 手动刷新，不进启动后台刷新任务。

**Non-Goals（后续阶段）：**
- 调仓 diff、股数口径信号计算、跨基金抱团趋势聚合。
- markdown 报告导出。
- 基金净值/估值/规模展示。
- 个股侧十大流通股东反查。

## Decisions

### D1. jjcc HTML 用正则解析，不引入 HTML 解析依赖
表格行模式固定（`<tr><td>序号</td>…<a>代码</a>…<a>名称</a>…<td class='tor'>数值</td>…`），已用同模式的正则在真实数据上验证。避免为一张结构稳定的表引入 `scraper` 及其整棵依赖树。解析失败（上游改版）时返回中文错误，不影响已落库数据。
**注意**：股票代码按 `\d{5,6}` 匹配——基金可持港股通标的（5 位代码，如 00700），只匹配 6 位会漏。

### D2. 单次抓取解析当年**全部**期次并全部落库，展示只取最新一期
jjcc 按年返回该年所有已披露期次（1 个 HTTP 请求）。全部 upsert 的成本为零，却为后续「调仓 diff」免费积累历史。若当年尚无任何期次（年初 1~4 月季报未出），自动回退拉上一年。「最新一期」= 已解析各表中报告期最大者；年报/中报全量表也只展示前 10 行（表本身按权重降序）。

### D3. 数据模型：两张新表，migration 只追加
```sql
CREATE TABLE IF NOT EXISTS watched_funds (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    fund_code TEXT NOT NULL UNIQUE,          -- 6 位基金代码
    fund_name TEXT NOT NULL,
    fund_type TEXT,                          -- 混合型-偏股 等，来自搜索接口
    created_at TEXT NOT NULL DEFAULT (datetime('now', 'localtime')),
    last_refreshed_at TEXT
);
CREATE TABLE IF NOT EXISTS fund_holding_snapshots (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    fund_code TEXT NOT NULL,
    report_date TEXT NOT NULL,               -- 报告期截止日 YYYY-MM-DD（由表标题推断：Q1=03-31…Q4=12-31）
    stock_code TEXT NOT NULL,                -- A股6位 / 港股5位
    stock_name TEXT NOT NULL,
    weight_pct REAL,                         -- 占净值比例(%)
    shares_wan REAL,                         -- 持股数（万股）——后续信号分析的关键字段
    market_value_wan REAL,                   -- 持仓市值（万元）
    holding_rank INTEGER,                    -- 表内排名
    created_at TEXT NOT NULL DEFAULT (datetime('now', 'localtime')),
    UNIQUE(fund_code, report_date, stock_code)
);
```
删除关注基金时**保留**其历史快照（数据地基，不随关注状态销毁）；重新添加即恢复完整历史。

### D4. IPC 命令面（`commands/funds.rs`，注册进 `lib.rs` `invoke_handler!`）
| 命令 | 参数(camelCase) | 行为 |
|---|---|---|
| `search_funds` | `keyword` | 调 fundsuggest，返回候选列表（不落库） |
| `add_watched_fund` | `fundCode, fundName, fundType` | 插入 watched_funds（重复报中文错误），随后自动做一次首刷 |
| `remove_watched_fund` | `fundCode` | 删除关注（保留快照） |
| `get_watched_funds` | — | 列表 + 每只基金最新 report_date |
| `refresh_fund_holdings` | `fundCode` | 抓取→解析→upsert→更新 last_refreshed_at→返回最新一期持仓 |
| `get_fund_holdings` | `fundCode` | 只读库：最新一期全部行（前端截前 10 展示） |

网络请求为 async（复用 reqwest/tokio）；**先完成 HTTP 与解析，再 `db.conn.lock()` 做 upsert**，绝不跨 `.await` 持锁（遵守 CLAUDE.md）。错误 `Result<T, String>` 中文文案。

### D5. 前端：`fundStore` + `Funds` 页，路由 `/funds`
- `src/stores/fundStore.ts`：照现有 store 形状（loading / error / invoke 封装），组件只消费 store。
- `src/pages/Funds/`：关注基金卡片列表；「添加基金」弹窗内做防抖搜索下拉；卡片展开显示持仓表（截止日期 + 前十大：代码/名称/权重/**股数**/市值）+ 每卡「刷新」按钮。
- `App.tsx` 加 `<Route path="/funds">`，侧边导航加「基金跟踪」入口。
- `src/types/index.ts` 新增 `WatchedFund` / `FundHolding` / `FundSearchResult`，字段 snake_case 与 Rust struct 一致。

### D6. 多份额（A/C）不做归一
同一基金的 A/C 份额代码不同但持仓相同。v1 由用户自行选择添加哪个份额，不做份额合并/去重——搜索结果中已显示完整名称（含 A/C 后缀），用户可辨别。

## Risks / Trade-offs

- [非公开接口变更/限流] → 快照落库天然降级：刷新失败时展示上次数据 + 中文警告；解析用防御式正则，失败即报错不写脏数据。频率本身极低（手动、季度级）。
- [HTML 表结构改版] → 解析逻辑集中在 `fund_data.rs` 单一函数，配单元测试（用真实响应片段做 fixture），改版时修一处。
- [报告期语义粗化] v1 用「表标题→季度末日期」推断 report_date，未区分数据出自季报还是年报全量（Q4 表两种来源同日期）。对"最新前十大"展示无影响；后续做 diff 时如需区分来源，可加列（migration 追加即可）。
- [港股代码与 A 股代码域重叠风险] 5/6 位纯数字文本存储，不解释市场；v1 只展示不关联行情，无冲突。后续关联行情时需要市场推断规则。

## Migration Plan

仅追加两张 `CREATE TABLE IF NOT EXISTS`，无既有表改动，天然幂等可回滚（新表不被旧代码引用）。版本号 `package.json` 与 `Cargo.toml` 同步递增。

## Open Questions

（无阻塞项。信号阈值、报告导出等已明确移入后续阶段。）
