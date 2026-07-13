export type Market = "US" | "CN" | "HK";
export type Currency = "USD" | "CNY" | "HKD";
export type TransactionType = "BUY" | "SELL" | "OPEN" | "PAY";

export interface Account {
  id: string;
  name: string;
  market: Market;
  description: string | null;
  created_at: string;
  updated_at: string;
}

export interface Category {
  id: string;
  name: string;
  color: string;
  icon: string;
  is_system: boolean;
  sort_order: number;
  created_at: string;
}

export interface Holding {
  id: string;
  account_id: string;
  symbol: string;
  name: string;
  market: Market;
  category_id: string | null;
  shares: number;
  avg_cost: number;
  currency: Currency;
  created_at: string;
  updated_at: string;
}

export interface Transaction {
  id: string;
  holding_id: string | null;
  account_id: string;
  symbol: string;
  name: string;
  market: Market;
  transaction_type: TransactionType;
  shares: number;
  price: number;
  total_amount: number;
  commission: number;
  currency: Currency;
  traded_at: string;
  notes: string | null;
  created_at: string;
}

export interface StockQuote {
  symbol: string;
  name: string;
  market: Market;
  current_price: number;
  previous_close: number;
  change: number;
  change_percent: number;
  high: number;
  low: number;
  volume: number;
  updated_at: string;
}

export interface HoldingWithQuote extends Holding {
  quote: StockQuote | null;
  market_value: number | null;
  total_cost: number | null;
  unrealized_pnl: number | null;
  unrealized_pnl_percent: number | null;
}

export interface ExchangeRates {
  usd_cny: number;
  usd_hkd: number;
  cny_hkd: number;
  updated_at: string;
}

export interface DailyPortfolioValue {
  id: number;
  date: string;
  total_cost: number;
  total_value: number;
  us_cost: number;
  us_value: number;
  cn_cost: number;
  cn_value: number;
  hk_cost: number;
  hk_value: number;
  exchange_rates: string;
  daily_pnl: number;
  cumulative_pnl: number;
}

// Phase 3: Dashboard types
export interface DashboardSummary {
  total_market_value: number;
  total_cost: number;
  total_pnl: number;
  total_pnl_percent: number;
  daily_pnl: number;
  us_market_value: number;
  cn_market_value: number;
  hk_market_value: number;
  exchange_rates: ExchangeRates;
  base_currency: string;
}

export interface HoldingDetail {
  id: string;
  account_id: string;
  account_name: string;
  symbol: string;
  name: string;
  market: string;
  category_name: string;
  category_color: string;
  shares: number;
  avg_cost: number;
  current_price: number;
  market_value: number;
  cost_value: number;
  pnl: number;
  pnl_percent: number | null;
  daily_pnl: number;
  currency: Currency;
  /** Market value normalised to USD for cross-currency sorting. */
  market_value_usd: number;
}

// Phase 3: Statistics types
export interface PieSlice {
  name: string;
  value: number;
  color?: string | null;
}

export interface PnlItem {
  symbol: string;
  name: string;
  pnl: number;
  pnl_percent: number | null;
  market_value: number;
}

export interface StatisticsOverview {
  total_market_value: number;
  total_cost: number;
  total_pnl: number;
  total_pnl_percent: number;
  market_distribution: PieSlice[];
  category_distribution: PieSlice[];
  account_distribution: PieSlice[];
  stock_distribution: PieSlice[];
  top_gainers: PnlItem[];
  top_losers: PnlItem[];
}

export interface MarketStatistics {
  market: string;
  total_market_value: number;
  total_cost: number;
  total_pnl: number;
  total_pnl_percent: number;
  account_distribution: PieSlice[];
  category_distribution: PieSlice[];
  stock_distribution: PieSlice[];
  holdings: HoldingDetail[];
}

export interface AccountStatistics {
  account_id: string;
  account_name: string;
  market: string;
  total_market_value: number;
  total_cost: number;
  total_pnl: number;
  total_pnl_percent: number;
  category_distribution: PieSlice[];
  stock_distribution: PieSlice[];
  holdings: HoldingDetail[];
}

export interface CategoryStatistics {
  category_id: string;
  category_name: string;
  category_color: string;
  total_market_value: number;
  total_cost: number;
  total_pnl: number;
  total_pnl_percent: number;
  market_distribution: PieSlice[];
  holdings: HoldingDetail[];
}

export interface CreateAccountPayload {
  name: string;
  market: Market;
  description?: string;
}

export interface UpdateAccountPayload {
  id: string;
  name: string;
  market: Market;
  description?: string;
}

export interface CreateCategoryPayload {
  name: string;
  color: string;
  icon: string;
  sortOrder?: number;
}

export interface UpdateCategoryPayload {
  id: string;
  name: string;
  color: string;
  icon: string;
  sortOrder?: number;
}

export interface CreateHoldingPayload {
  accountId: string;
  symbol: string;
  name: string;
  market: Market;
  categoryId?: string;
  shares: number;
  avgCost: number;
  currency: Currency;
}

export interface UpdateHoldingPayload {
  id: string;
  accountId: string;
  symbol: string;
  name: string;
  market: Market;
  categoryId?: string;
  shares: number;
  avgCost: number;
  currency: Currency;
}

// Phase 4: Performance types
export interface PerformanceSummary {
  start_date: string;
  end_date: string;
  start_value: number;
  end_value: number;
  total_return: number;
  annualized_return: number;
  total_pnl: number;
  max_drawdown: number;
  volatility: number;
  sharpe_ratio: number;
  /** Daily return series computed from the same DB query as the summary. */
  return_series: ReturnDataPoint[];
}

export interface ReturnDataPoint {
  date: string;
  cumulative_return: number;
  daily_return: number;
  portfolio_value: number;
  daily_pnl: number;
}

export interface DrawdownPoint {
  date: string;
  drawdown: number;
}

export interface DrawdownAnalysis {
  max_drawdown: number;
  peak_date: string;
  trough_date: string;
  recovery_date: string | null;
  drawdown_duration: number;
  recovery_duration: number | null;
  drawdown_series: DrawdownPoint[];
}

export interface AttributionItem {
  name: string;
  pnl: number;
  contribution_percent: number;
  weight: number;
}

export interface ReturnAttribution {
  total_pnl: number;
  by_market: AttributionItem[];
  by_category: AttributionItem[];
  by_holding: AttributionItem[];
}

export interface MonthlyReturn {
  year: number;
  month: number;
  return_rate: number;
  pnl: number;
  start_value: number;
  end_value: number;
}

export interface HoldingPerformance {
  symbol: string;
  name: string;
  market: string;
  category_name: string;
  return_rate: number;
  pnl: number;
  start_value: number;
  end_value: number;
}

export interface RiskMetrics {
  daily_volatility: number;
  annualized_volatility: number;
  sharpe_ratio: number;
  risk_free_rate: number;
  max_drawdown: number;
  calmar_ratio: number;
}

export interface CreateTransactionPayload {
  accountId: string;
  symbol: string;
  name: string;
  market: Market;
  transactionType: TransactionType;
  shares: number;
  price: number;
  totalAmount: number;
  commission: number;
  currency: Currency;
  tradedAt: string;
  notes?: string;
}

export interface UpdateTransactionPayload {
  id: string;
  accountId: string;
  symbol: string;
  name: string;
  market: Market;
  transactionType: TransactionType;
  shares: number;
  price: number;
  totalAmount: number;
  commission: number;
  currency: Currency;
  tradedAt: string;
  notes?: string;
}

// Phase 5: Quarterly Analysis types
export interface QuarterlySnapshot {
  id: string;
  quarter: string;
  snapshot_date: string;
  total_value: number;
  total_cost: number;
  total_pnl: number;
  us_value: number;
  us_cost: number;
  cn_value: number;
  cn_cost: number;
  hk_value: number;
  hk_cost: number;
  exchange_rates: string;
  overall_notes: string | null;
  created_at: string;
  holding_count: number;
}

export interface QuarterlyHoldingSnapshot {
  id: string;
  quarterly_snapshot_id: string;
  account_id: string;
  account_name: string;
  symbol: string;
  name: string;
  market: string;
  category_name: string;
  category_color: string;
  shares: number;
  avg_cost: number;
  close_price: number;
  market_value: number;
  cost_value: number;
  pnl: number;
  pnl_percent: number | null;
  weight: number;
  notes: string | null;
}

export interface QuarterlySnapshotDetail {
  snapshot: QuarterlySnapshot;
  holdings: QuarterlyHoldingSnapshot[];
  holding_changes: HoldingChanges | null;
  previous_quarter: string | null;
}

export interface ComparisonOverview {
  q1_total_value: number;
  q2_total_value: number;
  value_change: number;
  value_change_percent: number;
  q1_total_cost: number;
  q2_total_cost: number;
  q1_pnl: number;
  q2_pnl: number;
  q1_holding_count: number;
  q2_holding_count: number;
}

export interface MarketComparison {
  market: string;
  q1_value: number;
  q2_value: number;
  value_change: number;
  value_change_percent: number;
  q1_cost: number;
  q2_cost: number;
  q1_pnl: number;
  q2_pnl: number;
}

export interface CategoryComparison {
  category_name: string;
  category_color: string;
  q1_value: number;
  q2_value: number;
  value_change: number;
  value_change_percent: number;
  q1_cost: number;
  q2_cost: number;
  q1_pnl: number;
  q2_pnl: number;
}

export interface HoldingChangeItem {
  symbol: string;
  name: string;
  market: string;
  category_name: string;
  q1_shares: number | null;
  q2_shares: number | null;
  q1_value: number | null;
  q2_value: number | null;
  shares_change: number;
  value_change: number;
}

export interface HoldingChanges {
  new_holdings: HoldingChangeItem[];
  closed_holdings: HoldingChangeItem[];
  increased: HoldingChangeItem[];
  decreased: HoldingChangeItem[];
  unchanged: HoldingChangeItem[];
}

export interface QuarterComparison {
  quarter1: string;
  quarter2: string;
  overview: ComparisonOverview;
  by_market: MarketComparison[];
  by_category: CategoryComparison[];
  holding_changes: HoldingChanges;
}

export interface HoldingNoteHistory {
  quarter: string;
  snapshot_date: string;
  shares: number;
  avg_cost: number;
  close_price: number;
  pnl_percent: number | null;
  notes: string;
}

export interface QuarterlyNotesSummary {
  snapshot_id: string;
  quarter: string;
  snapshot_date: string;
  overall_notes: string;
  total_value: number;
  total_pnl: number;
}

export interface QuarterlyTrends {
  quarters: string[];
  total_values: number[];
  total_costs: number[];
  total_pnls: number[];
  market_values: Record<string, number[]>;
  category_values: Record<string, number[]>;
  holding_counts: number[];
}

/** Per-stock summary of transactions within a quarter. */
export interface StockTransactionGroup {
  symbol: string;
  name: string;
  market: Market;
  currency: Currency;
  buy_count: number;
  sell_count: number;
  total_buy_shares: number;
  total_sell_shares: number;
  total_buy_amount: number;
  total_sell_amount: number;
  transactions: Transaction[];
}

// Phase 6: Import/Export types
export interface ExportFilters {
  market?: string;
  account_id?: string;
  category_id?: string;
}

export interface ImportError {
  row: number;
  column: string;
  message: string;
}

export interface ImportPreview {
  total_rows: number;
  valid_rows: number;
  error_rows: ImportError[];
  preview_data: Record<string, unknown>[];
  column_mapping: Record<string, string>;
}

export interface ImportData {
  data_type: string;
  rows: Record<string, unknown>[];
  column_mapping: Record<string, string>;
  account_id: string;
}

export interface ImportSkipped {
  row: number;
  symbol: string;
  reason: string;
}

export interface ImportResult {
  imported_count: number;
  skipped_count: number;
  skipped_rows: ImportSkipped[];
  errors: ImportError[];
}

// Phase 6: Price Alerts
export type AlertType =
  | "PRICE_ABOVE"
  | "PRICE_BELOW"
  | "CHANGE_ABOVE"
  | "CHANGE_BELOW"
  | "PNL_ABOVE"
  | "PNL_BELOW";

export interface PriceAlert {
  id: string;
  holding_id: string | null;
  symbol: string;
  name: string;
  market: Market;
  alert_type: AlertType;
  threshold: number;
  is_active: boolean;
  is_triggered: boolean;
  triggered_at: string | null;
  created_at: string;
}

export interface TriggeredAlert {
  alert: PriceAlert;
  current_value: number;
  message: string;
}

// Phase 6: Review types
export interface QuarterlyHoldingStatus {
  snapshot_id: string;
  quarter: string;
  shares: number;
  avg_cost: number;
  close_price: number;
  pnl_percent: number | null;
  notes: string | null;
  decision_quality: "correct" | "wrong" | "pending" | null;
}

export interface HoldingReview {
  symbol: string;
  name: string;
  market: Market;
  is_current_holding: boolean;
  quarterly_timeline: QuarterlyHoldingStatus[];
}

export interface DecisionStatistics {
  total_decisions: number;
  correct_count: number;
  wrong_count: number;
  pending_count: number;
  accuracy_rate: number;
}

// Phase 6: AI Config
export interface AiConfig {
  provider: string;
  api_key: string;
  model: string;
  base_url: string | null;
  system_prompt: string;
}

// Quote Provider Config
export type QuoteProvider = "yahoo" | "eastmoney" | "xueqiu";

export interface QuoteProviderConfig {
  us_provider: QuoteProvider;
  hk_provider: QuoteProvider;
  cn_provider: QuoteProvider;
  xueqiu_cookie?: string | null;
  xueqiu_u?: string | null;
  /** A-share: adjust avg_cost on SELL and dividend. Default true. */
  cn_adjust_sell_pay_cost?: boolean;
  /** US stock: adjust avg_cost on SELL and dividend. Default false. */
  us_adjust_sell_pay_cost?: boolean;
  /** HK stock: adjust avg_cost on SELL and dividend. Default false. */
  hk_adjust_sell_pay_cost?: boolean;
}

// Options Management types
export interface OptionRecord {
  id: string;
  account_id: string;
  option_symbol: string;
  underlying: string;
  expiry_date: string;
  strike_price: number;
  option_type: "P" | "C";
  action: "SELL" | "BUY";
  code: string;
  quantity: number;
  price: number;
  amount: number;
  commission: number;
  fee: number;
  traded_at: string | null;
  settled_at: string | null;
  created_at: string;
}

export interface OptionContract {
  id: string;
  option_symbol: string;
  underlying: string;
  expiry_date: string;
  strike_price: number;
  option_type: "P" | "C";
  contracts: number;
  open_price: number;
  open_amount: number;
  commission: number;
  traded_at: string | null;
  close_price: number | null;
  close_code: string | null;
  status: "active" | "expired" | "assigned" | "closed";
  account_id: string;
}

export interface ExpiredOptionStats {
  total_contracts: number;
  assigned_contracts: number;
  expired_contracts: number;
  assignment_ratio: number;
}

export interface SellPutSimulation {
  underlying: string;
  contracts: PutContractSimulation[];
  total_cash_needed: number;
}

export interface PutContractSimulation {
  option_symbol: string;
  strike_price: number;
  contracts: number;
  would_be_assigned: boolean;
  cash_needed: number;
}

export interface SellCallSimulation {
  underlying: string;
  contracts: CallContractSimulation[];
  total_shares_needed: number;
}

export interface CallContractSimulation {
  option_symbol: string;
  strike_price: number;
  contracts: number;
  would_be_assigned: boolean;
  shares_needed: number;
}

export interface ImportOptionsResult {
  imported: number;
  skipped: number;
  errors: string[];
}

export interface StockSplit {
  id: number;
  stock_code: string;
  split_date: string;
  ratio_from: number;
  ratio_to: number;
  created_at: string;
}

export interface OptionShareLot {
  id: number;
  stock_code: string;
  shares_per_contract: number;
  created_at: string;
}

export interface StockPriceInput {
  symbol: string;
  price: number;
}

// ===== Fund tracking (组合 + 仓位模型, mirrors src-tauri/src/models/portfolio.rs) =====

export interface Portfolio {
  id: string;
  name: string;
  source_type: "FUND" | "MANUAL";
  fund_code: string | null;
  fund_type: string | null;
  latest_as_of_date: string | null;
  last_refreshed_at: string | null;
  created_at: string;
  updated_at: string;
}

export interface PortfolioPosition {
  id: number;
  portfolio_id: string;
  as_of_date: string;
  stock_code: string;
  stock_name: string;
  weight_pct: number | null;
  shares_wan: number | null;
  market_value_wan: number | null;
  position_rank: number | null;
  created_at: string;
}

export interface FundSearchResult {
  fund_code: string;
  fund_name: string;
  fund_type: string;
}

export interface PortfolioVersion {
  as_of_date: string;
  row_count: number;
  coverage: "FULL" | "PARTIAL";
}

export interface PositionDiffItem {
  stock_code: string;
  stock_name: string;
  change_type: "NEW" | "EXITED" | "INCREASED" | "DECREASED" | "UNCHANGED";
  basis: "shares" | "weight";
  from_shares_wan: number | null;
  to_shares_wan: number | null;
  shares_delta_wan: number | null;
  shares_delta_pct: number | null;
  from_weight_pct: number | null;
  to_weight_pct: number | null;
  weight_delta_pp: number | null;
  to_market_value_wan: number | null;
  from_rank: number | null;
  to_rank: number | null;
}

export interface PositionDiff {
  from_version: PortfolioVersion;
  to_version: PortfolioVersion;
  items: PositionDiffItem[];
}

// ===== Fund drawdown signal (最大回撤定投信号, mirrors models/portfolio.rs) =====

/** 定投信号档位：正常 / 接近历史大底 / 建议开启定投 */
export type FundSignalState = "NORMAL" | "APPROACHING" | "BUY_ZONE";

/** 逐日回撤点（复用 performance 的 DrawdownPoint；drawdown 为负百分比） */
export interface FundDrawdownPoint {
  date: string;
  drawdown: number;
}

/** 某窗口口径下的最大回撤概要（max_drawdown 为负百分比） */
export interface DrawdownWindow {
  label: string;
  max_drawdown: number;
  peak_date: string;
  trough_date: string;
  recovery_date: string | null;
}

export interface FundDrawdownAnalysis {
  fund_code: string;
  fund_type: string | null;
  start_date: string;
  latest_date: string;
  latest_adjusted_nav: number;
  /** 最新单位净值（平台申购净值，供对照；可能为 null） */
  latest_unit_nav: number | null;
  peak_nav: number;
  /** 全历史最大回撤 HMDD（负百分比） */
  max_drawdown: number;
  peak_date: string;
  trough_date: string;
  recovery_date: string | null;
  /** 当前回撤 CDD（负百分比） */
  current_drawdown: number;
  /** 历史最大回撤信号线（复权净值） L = 峰值 × (1 − |HMDD|) */
  threshold_nav: number;
  /** 信号线对应的单位净值（跌到此值即触发定投，假设期间无分红；可能为 null） */
  threshold_unit_nav: number | null;
  /** 距触线还需下跌的百分比（正=尚需下跌；≤0=已在触线下方） */
  distance_to_signal_pct: number;
  signal_state: FundSignalState;
  /** 接近区系数（默认 0.9） */
  approaching_ratio: number;
  windows: DrawdownWindow[];
  history_too_short: boolean;
  applicability_note: string | null;
  drawdown_series: FundDrawdownPoint[];
}

