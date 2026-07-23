import { useMemo } from "react";
import { Card, Tabs } from "antd";
import PieChart from "../../components/charts/PieChart";
import type { DashboardSummary, ExchangeRates, HoldingDetail, PieSlice } from "../../types";

const ON_EXCHANGE_FUND_CATEGORY = "场内基金";
const OTHER_LABEL = "其他";
const SMALL_THRESHOLD_PCT = 2; // 占比小于该阈值的非基金个股归入「其他」

const currencySymbol: Record<string, string> = { USD: "$", CNY: "¥", HKD: "HK$" };

interface Props {
  summary: DashboardSummary | null;
  holdings: HoldingDetail[];
}

// 将 USD 口径市值换算为基准货币市值。比率不变，仅金额对齐显示币种。
function toBaseUsd(valueUsd: number, base: string, rates: ExchangeRates | undefined): number {
  if (!rates || base === "USD") return valueUsd;
  if (base === "CNY") return valueUsd * rates.usd_cny;
  if (base === "HKD") return valueUsd * rates.usd_hkd;
  return valueUsd;
}

export default function QuickCharts({ summary, holdings }: Props) {
  const marketData = useMemo<PieSlice[]>(() => {
    if (!summary) return [];
    return [
      { name: "🇺🇸 美股", value: parseFloat(summary.us_market_value.toFixed(2)) },
      { name: "🇨🇳 A股", value: parseFloat(summary.cn_market_value.toFixed(2)) },
      { name: "🇭🇰 港股", value: parseFloat(summary.hk_market_value.toFixed(2)) },
    ].filter((d) => d.value > 0);
  }, [summary]);

  // 持仓分布：按个股聚合，场内基金统一合并为「场内基金」切片，
  // 其余占比 <2% 的个股合并为「其他」。两者互不干扰。
  const holdingData = useMemo<PieSlice[]>(() => {
    const base = summary?.base_currency ?? "USD";
    const rates = summary?.exchange_rates;
    // 先按 USD 口径聚合（跨币种可加），再统一换算为基准货币用于显示。
    const valid = holdings.filter((h) => h.market_value_usd > 0);
    const totalUsd = valid.reduce((sum, h) => sum + h.market_value_usd, 0);
    if (totalUsd <= 0) return [];

    // 场内基金统一合并；其余按 symbol 聚合为单只个股。
    let fundValueUsd = 0;
    const stockMap = new Map<string, { name: string; value: number }>();
    for (const h of valid) {
      if (h.category_name === ON_EXCHANGE_FUND_CATEGORY) {
        fundValueUsd += h.market_value_usd;
      } else {
        const entry = stockMap.get(h.symbol);
        if (entry) {
          entry.value += h.market_value_usd;
        } else {
          stockMap.set(h.symbol, { name: h.name || h.symbol, value: h.market_value_usd });
        }
      }
    }

    const slices: PieSlice[] = [];
    let otherValueUsd = 0;
    for (const { name, value } of stockMap.values()) {
      const pct = (value / totalUsd) * 100;
      if (pct < SMALL_THRESHOLD_PCT) {
        otherValueUsd += value;
      } else {
        slices.push({ name, value: parseFloat(toBaseUsd(value, base, rates).toFixed(2)) });
      }
    }
    if (fundValueUsd > 0) {
      slices.push({ name: ON_EXCHANGE_FUND_CATEGORY, value: parseFloat(toBaseUsd(fundValueUsd, base, rates).toFixed(2)) });
    }
    if (otherValueUsd > 0) {
      slices.push({ name: OTHER_LABEL, value: parseFloat(toBaseUsd(otherValueUsd, base, rates).toFixed(2)) });
    }

    // 按 value 降序，但「其他」固定置末。
    return slices.sort((a, b) => {
      if (a.name === OTHER_LABEL) return 1;
      if (b.name === OTHER_LABEL) return -1;
      return b.value - a.value;
    });
  }, [holdings, summary]);

  if (!summary) return null;

  const total = summary.total_market_value.toFixed(0);
  const currency = summary.base_currency;
  const centerText = `${currencySymbol[currency] ?? ""}${Number(total).toLocaleString()}`;

  const TAB_EMPTY = (
    <div style={{ textAlign: "center", color: "#999", padding: 40 }}>
      暂无数据
    </div>
  );

  const tabItems = [
    {
      key: "holding",
      label: "持仓分布",
      children: holdingData.length > 0 ? (
        <PieChart data={holdingData} height={640} centerText={centerText} currencyCode={currency} />
      ) : TAB_EMPTY,
    },
    {
      key: "market",
      label: "市场分布",
      children: marketData.length > 0 ? (
        <PieChart data={marketData} height={640} centerText={centerText} currencyCode={currency} />
      ) : TAB_EMPTY,
    },
  ];

  return (
    <Card className="mt-4" size="small">
      <Tabs defaultActiveKey="holding" items={tabItems} destroyInactiveTabPane />
    </Card>
  );
}
