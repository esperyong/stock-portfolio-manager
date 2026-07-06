# 任务：基金跟踪 v2——调仓 diff

## 1. 后端 diff 服务与模型

- [x] 1.1 在 `src-tauri/src/models/portfolio.rs` 追加 struct：`PortfolioVersion`（as_of_date/row_count/coverage）、`PositionDiff`（from_version/to_version/items）、`PositionDiffItem`（change_type/basis/股数与权重变化字段/排名，serde snake_case）
- [x] 1.2 新增 `src-tauri/src/services/position_diff.rs`：纯函数 `infer_coverage(as_of_date, row_count)`（3/9 月恒 PARTIAL；6/12 月按行数≥35 判 FULL）与 `compute_diff(from_rows, to_rows)`（股数主口径、epsilon 0.001 持平、股数缺失回退权重并标记 basis、类别排序+类内按 |Δweight_pp| 降序）
- [x] 1.3 为 `position_diff.rs` 写单元测试：全量对全量（新建/清仓/加/减/持平各至少一例）、口径推断边界（3/9 月、6/12 月行数 34/35、全量对部分）、股数缺失回退权重、双侧缺失、HK 前导零代码、共同持股跨口径对比不受影响
- [x] 1.4 在 `src-tauri/src/commands/portfolios.rs` 追加只读命令 `get_portfolio_versions`（按日期降序含 coverage）与 `get_portfolio_diff`（fromDate/toDate 可省略默认最新两期；不足两期报中文错误「该组合尚不足两期持仓数据，无法对比」）
- [x] 1.5 在 `lib.rs` `invoke_handler![...]` 注册两个新命令；Docker 环境（见 tauri-test-on-this-ec2 方案）`cargo test` 全绿
- [x] 1.6 `fund_data.rs` 刷新改为固定双年拉取（当年+上一年，见 design D7）：保证新添加基金首刷即有多期版本可对比；重跑 `cargo test` 全绿

## 2. 前端

- [x] 2.1 `src/types/index.ts` 追加镜像类型 `PortfolioVersion` / `PositionDiff` / `PositionDiffItem`（字段与 Rust struct 完全一致）
- [x] 2.2 `src/stores/portfolioStore.ts` 追加 `fetchVersions(portfolioId)` 与 `fetchDiff(portfolioId, fromDate?, toDate?)`（照现有 store 形状，错误转中文字符串）
- [x] 2.3 新增 `src/pages/Funds/PositionDiffView.tsx`：期次双下拉（默认最新两期）→ 分类明细表（变动类型 Tag 按口径矩阵渲染措辞：FULL→FULL 显示新建仓/清仓，涉及 PARTIAL 显示新进披露/退出披露；股数 from→to 及 ±%；权重变化 pp；basis=weight 时注明按权重估算）；涉及 PARTIAL 时顶部提示条
- [x] 2.4 组合卡片展开区改双页签：「最新持仓」（原表）/「调仓」（新组件）；不足两期时调仓页签显示引导文案而非报错弹窗；`npx tsc --noEmit` 通过

## 3. 端到端验证

- [ ] 3.1 `npm run tauri dev` 实测（开发机）：对已积累两期以上的基金（如 163417 有 2025 全年 4 期）打开调仓视图 → 默认最新两期对比正确；切换 2025-06-30 vs 2025-12-31（全量对全量）出现真实清仓/新建仓标签；对比含 2026-03-31（部分披露）时标签变为新进/退出披露且有提示条；单期组合显示引导文案
- [x] 3.2 确认调仓视图全程零网络请求（断网可用）；v1 既有功能（最新持仓表、刷新、删除）行为不变
