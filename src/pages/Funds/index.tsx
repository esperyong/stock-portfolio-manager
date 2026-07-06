import { useEffect, useState } from "react";
import {
  Alert,
  Button,
  Card,
  Empty,
  Popconfirm,
  Space,
  Table,
  Tabs,
  Tag,
  Typography,
  message,
} from "antd";
import {
  DeleteOutlined,
  DownOutlined,
  PlusOutlined,
  ReloadOutlined,
  UpOutlined,
} from "@ant-design/icons";
import { usePortfolioStore } from "../../stores/portfolioStore";
import AddFundModal from "./AddFundModal";
import PositionDiffView from "./PositionDiffView";
import type { Portfolio, PortfolioPosition } from "../../types";

const { Title, Text } = Typography;

const TOP_N = 10;

function formatDateTime(value: string | null): string {
  if (!value) return "—";
  const d = new Date(value);
  return Number.isNaN(d.getTime()) ? value : d.toLocaleString("zh-CN", { hour12: false });
}

function formatNumber(value: number | null, digits: number): string {
  if (value === null || value === undefined) return "—";
  return value.toLocaleString("zh-CN", {
    minimumFractionDigits: digits,
    maximumFractionDigits: digits,
  });
}

const positionColumns = [
  {
    title: "股票代码",
    dataIndex: "stock_code",
    key: "stock_code",
    width: 100,
    render: (code: string) => <span style={{ fontFamily: "monospace" }}>{code}</span>,
  },
  { title: "股票名称", dataIndex: "stock_name", key: "stock_name" },
  {
    title: "占净值比",
    dataIndex: "weight_pct",
    key: "weight_pct",
    align: "right" as const,
    render: (v: number | null) => (v === null ? "—" : `${v.toFixed(2)}%`),
  },
  {
    title: "持股数（万股）",
    dataIndex: "shares_wan",
    key: "shares_wan",
    align: "right" as const,
    render: (v: number | null) => formatNumber(v, 2),
  },
  {
    title: "持仓市值（万元）",
    dataIndex: "market_value_wan",
    key: "market_value_wan",
    align: "right" as const,
    render: (v: number | null) => formatNumber(v, 2),
  },
];

function PortfolioCard({ portfolio }: { portfolio: Portfolio }) {
  const { positions, refreshingId, refreshPortfolio, deletePortfolio, fetchPositions } =
    usePortfolioStore();
  const [expanded, setExpanded] = useState(false);
  const [loadingPositions, setLoadingPositions] = useState(false);

  const rows = positions[portfolio.id];
  const refreshing = refreshingId === portfolio.id;

  const handleToggle = async () => {
    const next = !expanded;
    setExpanded(next);
    if (next && rows === undefined) {
      setLoadingPositions(true);
      try {
        await fetchPositions(portfolio.id);
      } catch (err) {
        message.error(`仓位加载失败: ${err}`);
      } finally {
        setLoadingPositions(false);
      }
    }
  };

  const handleRefresh = async () => {
    try {
      await refreshPortfolio(portfolio.id);
      message.success("刷新成功");
    } catch (err) {
      message.error(String(err));
    }
  };

  const handleDelete = async () => {
    try {
      await deletePortfolio(portfolio.id);
      message.success("组合已删除");
    } catch (err) {
      message.error(`删除失败: ${err}`);
    }
  };

  const topRows = (rows ?? []).slice(0, TOP_N);

  return (
    <Card
      className="mb-4"
      title={
        <Space wrap>
          <span>{portfolio.name}</span>
          {portfolio.fund_code && (
            <Tag color="blue" style={{ fontFamily: "monospace" }}>
              {portfolio.fund_code}
            </Tag>
          )}
          {portfolio.fund_type && <Tag>{portfolio.fund_type}</Tag>}
        </Space>
      }
      extra={
        <Space>
          <Button
            icon={<ReloadOutlined />}
            size="small"
            loading={refreshing}
            onClick={handleRefresh}
          >
            刷新
          </Button>
          <Button
            icon={expanded ? <UpOutlined /> : <DownOutlined />}
            size="small"
            onClick={handleToggle}
          >
            {expanded ? "收起" : "展开"}
          </Button>
          <Popconfirm
            title="确认删除该基金组合？"
            description="将同时删除已保存的各期仓位（可重新添加后刷新取回）"
            onConfirm={handleDelete}
            okText="确认"
            cancelText="取消"
          >
            <Button icon={<DeleteOutlined />} size="small" danger>
              删除
            </Button>
          </Popconfirm>
        </Space>
      }
    >
      <Space size="large" wrap>
        <Text type="secondary">
          最新报告期：<Text strong>{portfolio.latest_as_of_date ?? "—"}</Text>
        </Text>
        <Text type="secondary">
          上次刷新：<Text strong>{formatDateTime(portfolio.last_refreshed_at)}</Text>
        </Text>
      </Space>
      {expanded && (
        <Tabs
          style={{ marginTop: 8 }}
          size="small"
          items={[
            {
              key: "positions",
              label: "最新持仓",
              children: (
                <div>
                  {topRows.length > 0 && (
                    <Text type="secondary" style={{ display: "block", marginBottom: 8 }}>
                      报告期截止：{topRows[0].as_of_date}（前 {topRows.length} 大重仓）
                    </Text>
                  )}
                  <Table<PortfolioPosition>
                    dataSource={topRows}
                    columns={positionColumns}
                    rowKey="id"
                    size="small"
                    loading={loadingPositions}
                    pagination={false}
                    locale={{
                      emptyText: (
                        <Empty
                          image={Empty.PRESENTED_IMAGE_SIMPLE}
                          description="暂无仓位数据，请点击「刷新」获取"
                        />
                      ),
                    }}
                  />
                </div>
              ),
            },
            {
              key: "diff",
              label: "调仓",
              children: <PositionDiffView portfolioId={portfolio.id} />,
            },
          ]}
        />
      )}
    </Card>
  );
}

export default function FundsPage() {
  const { portfolios, loading, error, fetchPortfolios } = usePortfolioStore();
  const [addOpen, setAddOpen] = useState(false);

  useEffect(() => {
    fetchPortfolios();
  }, [fetchPortfolios]);

  return (
    <div>
      <div className="flex justify-between items-center mb-4">
        <Title level={2} className="!mb-0">
          基金跟踪
        </Title>
        <Button type="primary" icon={<PlusOutlined />} onClick={() => setAddOpen(true)}>
          添加基金
        </Button>
      </div>

      {error && (
        <Alert type="error" showIcon message="组合列表加载失败" description={error} className="mb-4" />
      )}

      {!loading && portfolios.length === 0 ? (
        <Empty description="尚未跟踪任何基金，点击「添加基金」开始" style={{ marginTop: 80 }} />
      ) : (
        portfolios.map((p) => <PortfolioCard key={p.id} portfolio={p} />)
      )}

      <AddFundModal open={addOpen} onClose={() => setAddOpen(false)} />
    </div>
  );
}
