# 任务：基金跟踪 v1

## 1. 数据库与模型

- [ ] 1.1 在 `src-tauri/src/db/mod.rs` 的 `run_migrations()` 末尾追加 `watched_funds` 与 `fund_holding_snapshots` 两张表（`CREATE TABLE IF NOT EXISTS`，含 `UNIQUE(fund_code, report_date, stock_code)`），保持幂等
- [ ] 1.2 在 `src-tauri/src/models/` 新增基金相关 struct：`WatchedFund`、`FundHolding`、`FundSearchResult`（serde 序列化，snake_case 字段）
- [ ] 1.3 在 `src/types/index.ts` 新增镜像 TS 接口（字段名与 Rust struct 完全一致）

## 2. 后端服务与命令

- [ ] 2.1 新增 `src-tauri/src/services/fund_data.rs`：fundsuggest 搜索请求（带 Referer 头）+ 响应解析
- [ ] 2.2 同文件实现 jjcc 持仓抓取与解析：按年请求 → 按「N季度股票投资明细」切表 → 正则逐行提取（代码兼容 5/6 位、数值去千分位）→ 表标题推断 report_date（Q1=03-31、Q2=06-30、Q3=09-30、Q4=12-31）；当年无表时回退上一年
- [ ] 2.3 为 jjcc 解析写单元测试：用真实响应片段做 fixture，覆盖季报 10 行表、年报全量表、含港股代码行、空响应
- [ ] 2.4 新增 `src-tauri/src/commands/funds.rs`：`search_funds` / `add_watched_fund`（重复添加报中文错误，成功后自动首刷）/ `remove_watched_fund`（保留快照）/ `get_watched_funds`（含最新 report_date）/ `refresh_fund_holdings`（抓取→解析完成后再锁库 upsert，不跨 await 持锁）/ `get_fund_holdings`（只读库返回最新一期）
- [ ] 2.5 在 `src-tauri/src/lib.rs` 的 `invoke_handler![...]` 注册全部新命令，并声明 `mod`；`cd src-tauri && cargo test` 全绿

## 3. 前端

- [ ] 3.1 新增 `src/stores/fundStore.ts`：照现有 store 形状封装六个 invoke（loading/error 状态、错误转中文字符串）
- [ ] 3.2 新增 `src/pages/Funds/` 页面：关注基金卡片列表（名称/代码/类型/最新报告期/上次刷新时间）、每卡「刷新」「删除」按钮、展开显示前十大持仓表（截止日期 + 代码/名称/占净值比/持股数(万股)/市值(万元)，按权重降序，最多 10 行）
- [ ] 3.3 实现「添加基金」弹窗：输入防抖搜索 → 候选下拉（代码+完整名称+类型，可区分 A/C 份额）→ 选中确认添加
- [ ] 3.4 在 `App.tsx` 注册路由 `/funds`，侧边导航新增「基金跟踪」入口；`npx tsc --noEmit` 通过

## 4. 端到端验证

- [ ] 4.1 `npm run tauri dev` 实测：搜索「兴全」→ 添加 163415 → 自动首刷出前十大（与天天基金网页 ccmx_163415 页对照一致）→ 重复刷新无重复行 → 删除后重新添加历史仍在 → 断网时仍展示已落库数据且报中文警告
- [ ] 4.2 检查启动流程未新增任何基金接口请求（后台刷新任务不含基金拉取）
