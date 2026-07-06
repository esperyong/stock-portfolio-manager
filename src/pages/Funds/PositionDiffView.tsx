import { useEffect, useState } from "react";
import { Alert, Checkbox, Empty, Select, Space, Spin, Table, Tag, Typography, message } from "antd";
import { ArrowRightOutlined } from "@ant-design/icons";
import { usePortfolioStore } from "../../stores/portfolioStore";
import type { PositionDiff, PositionDiffItem, PortfolioVersion } from "../../types";

const { Text } = Typography;

interface Props {
  portfolioId: string;
}

/**
 * change_type 的展示措辞按方向由披露口径决定（design D3 语义矩阵）：
 * "出现"的确定性取决于起始侧口径（起始全量 → 才能断言新建仓），
 * "消失"的确定性取决于目标侧口径（目标全量 → 即可断言清仓）。
 */
function changeLabel(
  type: PositionDiffItem["change_type"],
  fromFull: boolean,
  toFull: boolean
): string {
  switch (type) {
    case "NEW":
      return fromFull ? "新建仓" : "新进披露";
    case "EXITED":
      return toFull ? "清仓" : "退出披露";
    case "INCREASED":
      return "加仓";
    case "DECREASED":
      return "减仓";
    default:
      return "持平";
  }
}

// A股习惯：红涨绿跌
function changeColor(type: PositionDiffItem["change_type"]): string {
  switch (type) {
    case "NEW":
      return "red";
    case "INCREASED":
      return "volcano";
    case "DECREASED":
      return "green";
    case "EXITED":
      return "cyan";
    default:
      return "default";
  }
}

function fmt(value: number | null, digits = 2): string {
  if (value === null || value === undefined) return "—";
  return value.toLocaleString("zh-CN", {
    minimumFractionDigits: digits,
    maximumFractionDigits: digits,
  });
}

function versionLabel(v: PortfolioVersion): string {
  return `${v.as_of_date}（${v.coverage === "FULL" ? "全量" : "前十大"}·${v.row_count}只）`;
}

export default function PositionDiffView({ portfolioId }: Props) {
  const { versions, diffs, fetchVersions, fetchDiff } = usePortfolioStore();
  const [loading, setLoading] = useState(false);
  const [fromDate, setFromDate] = useState<string | undefined>();
  const [toDate, setToDate] = useState<string | undefined>();
  const [showUnchanged, setShowUnchanged] = useState(false);

  const versionList = versions[portfolioId];
  const diff: PositionDiff | undefined = diffs[portfolioId];

  useEffect(() => {
    let cancelled = false;
    (async () => {
      setLoading(true);
      try {
        const vs = await fetchVersions(portfolioId);
        if (cancelled) return;
        if (vs.length >= 2) {
          // 默认对比最新两期（列表按日期降序）
          setFromDate(vs[1].as_of_date);
          setToDate(vs[0].as_of_date);
          await fetchDiff(portfolioId, vs[1].as_of_date, vs[0].as_of_date);
        }
      } catch (err) {
        if (!cancelled) message.error(String(err));
      } finally {
        if (!cancelled) setLoading(false);
      }
    })();
    return () => {
      cancelled = true;
    };
  }, [portfolioId, fetchVersions, fetchDiff]);

  const handleChangePeriod = async (nextFrom?: string, nextTo?: string) => {
    if (!nextFrom || !nextTo) return;
    if (nextFrom === nextTo) {
      message.warning("起止报告期不能相同");
      return;
    }
    setLoading(true);
    try {
      await fetchDiff(portfolioId, nextFrom, nextTo);
    } catch (err) {
      message.error(String(err));
    } finally {
      setLoading(false);
    }
  };

  if (!versionList) {
    return (
      <div style={{ textAlign: "center", padding: 24 }}>
        <Spin size="small" /> 加载版本列表…
      </div>
    );
  }

  if (versionList.length < 2) {
    return (
      <Empty
        image={Empty.PRESENTED_IMAGE_SIMPLE}
        description="该基金目前只有一期持仓数据，等下一个报告期披露并刷新后即可对比调仓"
      />
    );
  }

  const fromFull = diff !== undefined && diff.from_version.coverage === "FULL";
  const toFull = diff !== undefined && diff.to_version.coverage === "FULL";
  const bothFull = fromFull && toFull;
  const rows = (diff?.items ?? []).filter(
    (item) => showUnchanged || item.change_type !== "UNCHANGED"
  );
  const unchangedCount = (diff?.items ?? []).filter((i) => i.change_type === "UNCHANGED").length;

  const options = versionList.map((v) => ({ value: v.as_of_date, label: versionLabel(v) }));

  const columns = [
    {
      title: "变动",
      dataIndex: "change_type",
      key: "change_type",
      width: 100,
      render: (type: PositionDiffItem["change_type"]) => (
        <Tag color={changeColor(type)}>{changeLabel(type, fromFull, toFull)}</Tag>
      ),
    },
    {
      title: "股票代码",
      dataIndex: "stock_code",
      key: "stock_code",
      width: 100,
      render: (code: string) => <span style={{ fontFamily: "monospace" }}>{code}</span>,
    },
    { title: "股票名称", dataIndex: "stock_name", key: "stock_name" },
    {
      title: "持股数变化（万股）",
      key: "shares",
      align: "right" as const,
      render: (_: unknown, item: PositionDiffItem) => (
        <Space size={4}>
          <span>{fmt(item.from_shares_wan)}</span>
          <ArrowRightOutlined style={{ fontSize: 10, color: "#999" }} />
          <span>{fmt(item.to_shares_wan)}</span>
          {item.shares_delta_pct !== null && item.change_type !== "UNCHANGED" && (
            <Text type={item.shares_delta_pct >= 0 ? "danger" : "success"}>
              （{item.shares_delta_pct >= 0 ? "+" : ""}
              {item.shares_delta_pct.toFixed(1)}%）
            </Text>
          )}
          {item.basis === "weight" && <Tag>按权重估算</Tag>}
        </Space>
      ),
    },
    {
      title: "权重变化",
      key: "weight",
      align: "right" as const,
      render: (_: unknown, item: PositionDiffItem) => (
        <Space size={4}>
          <span>{item.from_weight_pct === null ? "—" : `${item.from_weight_pct.toFixed(2)}%`}</span>
          <ArrowRightOutlined style={{ fontSize: 10, color: "#999" }} />
          <span>{item.to_weight_pct === null ? "—" : `${item.to_weight_pct.toFixed(2)}%`}</span>
          {item.weight_delta_pp !== null && (
            <Text type={item.weight_delta_pp >= 0 ? "danger" : "success"}>
              （{item.weight_delta_pp >= 0 ? "+" : ""}
              {item.weight_delta_pp.toFixed(2)}pp）
            </Text>
          )}
        </Space>
      ),
    },
  ];

  return (
    <div>
      <Space wrap style={{ marginBottom: 12 }}>
        <Text type="secondary">对比期次：</Text>
        <Select
          size="small"
          style={{ minWidth: 230 }}
          value={fromDate}
          options={options}
          onChange={(v) => {
            setFromDate(v);
            handleChangePeriod(v, toDate);
          }}
        />
        <ArrowRightOutlined style={{ color: "#999" }} />
        <Select
          size="small"
          style={{ minWidth: 230 }}
          value={toDate}
          options={options}
          onChange={(v) => {
            setToDate(v);
            handleChangePeriod(fromDate, v);
          }}
        />
        {unchangedCount > 0 && (
          <Checkbox checked={showUnchanged} onChange={(e) => setShowUnchanged(e.target.checked)}>
            显示持平项（{unchangedCount}）
          </Checkbox>
        )}
      </Space>

      {diff && !bothFull && (
        <Alert
          type="info"
          showIcon
          style={{ marginBottom: 12 }}
          message="所选期次包含仅披露前十大的季报口径"
          description={[
            !fromFull &&
              "起始期为前十大口径：「新进披露」不一定是新建仓（此前可能已持有但未进前十）。",
            !toFull &&
              "目标期为前十大口径：「退出披露」不代表清仓（可能仍持有但跌出前十）。",
          ]
            .filter(Boolean)
            .join(" ")}
        />
      )}

      <Table<PositionDiffItem>
        dataSource={rows}
        columns={columns}
        rowKey="stock_code"
        size="small"
        loading={loading}
        pagination={false}
        locale={{
          emptyText: <Empty image={Empty.PRESENTED_IMAGE_SIMPLE} description="两期之间无持仓变动" />,
        }}
      />
    </div>
  );
}
