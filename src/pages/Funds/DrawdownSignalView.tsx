import { useEffect, useState } from "react";
import {
  Alert,
  Button,
  Descriptions,
  Empty,
  Space,
  Statistic,
  Table,
  Tag,
  Typography,
  message,
} from "antd";
import { ReloadOutlined } from "@ant-design/icons";
import ReactECharts from "echarts-for-react";
import { usePortfolioStore } from "../../stores/portfolioStore";
import { usePnlColor } from "../../hooks/usePnlColor";
import type { DrawdownWindow, FundSignalState } from "../../types";

const { Text } = Typography;

export const SIGNAL_META: Record<
  FundSignalState,
  { label: string; color: string; tag: "success" | "warning" | "default"; hint: string }
> = {
  BUY_ZONE: {
    label: "建议开启定投",
    color: "#389e0d",
    tag: "success",
    hint: "当前回撤已达历史最大回撤，接近历史大底 —— 朴素策略下开启定投大概率不亏",
  },
  APPROACHING: {
    label: "接近历史大底",
    color: "#d46b08",
    tag: "warning",
    hint: "当前回撤已达历史最大回撤的九成，留意定投时机",
  },
  NORMAL: {
    label: "正常",
    color: "#8c8c8c",
    tag: "default",
    hint: "当前回撤距历史大底尚远；净值回升越过信号线即对应「停止定投」",
  },
};

function pct(v: number | null | undefined, digits = 2): string {
  if (v === null || v === undefined) return "—";
  return `${v.toFixed(digits)}%`;
}

const windowColumns = [
  { title: "口径", dataIndex: "label", key: "label", width: 88 },
  {
    title: "最大回撤",
    dataIndex: "max_drawdown",
    key: "max_drawdown",
    align: "right" as const,
    render: (v: number) => <Text type="danger">{pct(v)}</Text>,
  },
  { title: "峰值日期", dataIndex: "peak_date", key: "peak_date" },
  { title: "谷值日期", dataIndex: "trough_date", key: "trough_date" },
  {
    title: "修复日期",
    dataIndex: "recovery_date",
    key: "recovery_date",
    render: (v: string | null) => v ?? <Text type="secondary">未修复</Text>,
  },
];

export default function DrawdownSignalView({ portfolioId }: { portfolioId: string }) {
  const { drawdowns, refreshingNavId, refreshFundNav, fetchFundDrawdown } = usePortfolioStore();
  const { lossColor } = usePnlColor();
  const [loading, setLoading] = useState(false);
  const [needRefresh, setNeedRefresh] = useState(false);

  const analysis = drawdowns[portfolioId];
  const refreshing = refreshingNavId === portfolioId;

  useEffect(() => {
    if (analysis) return;
    let cancelled = false;
    setLoading(true);
    fetchFundDrawdown(portfolioId)
      .catch(() => {
        // 尚未抓过净值：引导用户先刷新，而非报错
        if (!cancelled) setNeedRefresh(true);
      })
      .finally(() => {
        if (!cancelled) setLoading(false);
      });
    return () => {
      cancelled = true;
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [portfolioId]);

  const handleRefreshNav = async () => {
    try {
      setNeedRefresh(false);
      await refreshFundNav(portfolioId);
      message.success("净值已刷新");
    } catch (err) {
      message.error(String(err));
    }
  };

  const refreshBtn = (
    <Button
      icon={<ReloadOutlined />}
      size="small"
      loading={refreshing}
      onClick={handleRefreshNav}
    >
      刷新净值
    </Button>
  );

  if (loading && !analysis) {
    return <Text type="secondary">加载中…</Text>;
  }

  if (!analysis) {
    return (
      <Empty
        image={Empty.PRESENTED_IMAGE_SIMPLE}
        description={needRefresh ? "尚未抓取净值，点击刷新以计算回撤信号" : "暂无回撤数据"}
      >
        {refreshBtn}
      </Empty>
    );
  }

  const meta = SIGNAL_META[analysis.signal_state];
  const dates = analysis.drawdown_series.map((d) => d.date);
  const values = analysis.drawdown_series.map((d) => parseFloat(d.drawdown.toFixed(2)));
  const hmdd = parseFloat(analysis.max_drawdown.toFixed(2));
  const approaching = parseFloat((analysis.max_drawdown * analysis.approaching_ratio).toFixed(2));
  const current = parseFloat(analysis.current_drawdown.toFixed(2));

  const chartOption = {
    tooltip: {
      trigger: "axis",
      formatter: (params: { axisValue: string; value: number }[]) => {
        if (!params.length) return "";
        const p = params[0];
        return `${p.axisValue}<br/>回撤: <b>${p.value.toFixed(2)}%</b>`;
      },
    },
    grid: { left: "3%", right: "5%", bottom: "12%", containLabel: true },
    xAxis: {
      type: "category",
      data: dates,
      axisLabel: { rotate: 30, formatter: (v: string) => v.slice(0, 7) },
    },
    yAxis: {
      type: "value",
      scale: true,
      max: 0,
      axisLabel: { formatter: (v: number) => `${v.toFixed(0)}%` },
    },
    dataZoom: [{ type: "inside" }, { type: "slider", height: 16, bottom: 4 }],
    series: [
      {
        name: "回撤",
        type: "line",
        data: values,
        smooth: true,
        symbol: "none",
        lineStyle: { width: 1, color: lossColor },
        areaStyle: { color: lossColor, opacity: 0.25 },
        markLine: {
          silent: true,
          symbol: "none",
          data: [
            {
              yAxis: hmdd,
              lineStyle: { color: "#389e0d", type: "solid", width: 1.5 },
              label: { formatter: `历史最大回撤线 ${hmdd}%`, position: "insideEndTop" },
            },
            {
              yAxis: approaching,
              lineStyle: { color: "#d46b08", type: "dashed", width: 1 },
              label: { formatter: `接近线 ${approaching}%`, position: "insideEndBottom" },
            },
            {
              yAxis: current,
              lineStyle: { color: "#1677ff", type: "dotted", width: 1 },
              label: { formatter: `当前 ${current}%`, position: "insideStartTop" },
            },
          ],
        },
      },
    ],
  };

  return (
    <div>
      <Space align="center" wrap style={{ justifyContent: "space-between", width: "100%" }}>
        <Space align="center">
          <Tag color={meta.tag} style={{ fontSize: 14, padding: "2px 10px" }}>
            {meta.label}
          </Tag>
          <Text type="secondary">{meta.hint}</Text>
        </Space>
        {refreshBtn}
      </Space>

      <Space size="large" wrap style={{ marginTop: 12 }}>
        <Statistic
          title="当前回撤"
          value={analysis.current_drawdown}
          precision={2}
          suffix="%"
          valueStyle={{ color: meta.color }}
        />
        <Statistic
          title="历史最大回撤"
          value={analysis.max_drawdown}
          precision={2}
          suffix="%"
          valueStyle={{ color: "#cf1322" }}
        />
        <Statistic
          title={analysis.distance_to_signal_pct > 0 ? "距触线还需下跌" : "已在触线下方"}
          value={Math.abs(analysis.distance_to_signal_pct)}
          precision={1}
          suffix="%"
        />
        <Statistic title="信号线净值" value={analysis.threshold_nav} precision={4} />
        <Statistic title="最新净值(复权)" value={analysis.latest_adjusted_nav} precision={4} />
      </Space>

      {analysis.history_too_short && (
        <Alert
          type="warning"
          showIcon
          className="mt-3"
          message="该基金净值历史不足一年，最大回撤与信号的参考意义有限。"
        />
      )}
      {analysis.applicability_note && (
        <Alert type="info" showIcon className="mt-3" message={analysis.applicability_note} />
      )}

      <Descriptions size="small" column={3} className="mt-3">
        <Descriptions.Item label="峰值日期">{analysis.peak_date}</Descriptions.Item>
        <Descriptions.Item label="谷值日期">{analysis.trough_date}</Descriptions.Item>
        <Descriptions.Item label="修复日期">
          {analysis.recovery_date ?? <Text type="secondary">未修复</Text>}
        </Descriptions.Item>
        <Descriptions.Item label="净值区间">
          {analysis.start_date} ~ {analysis.latest_date}
        </Descriptions.Item>
      </Descriptions>

      <Text strong style={{ display: "block", marginTop: 8 }}>
        📉 水下回撤曲线（绿线=历史最大回撤触发线）
      </Text>
      <ReactECharts
        option={chartOption}
        style={{ height: 300, width: "100%" }}
        opts={{ renderer: "canvas" }}
      />

      <Text strong style={{ display: "block", marginTop: 8, marginBottom: 4 }}>
        多窗口对照
      </Text>
      <Table<DrawdownWindow>
        dataSource={analysis.windows}
        columns={windowColumns}
        rowKey="label"
        size="small"
        pagination={false}
      />
    </div>
  );
}
