import { useEffect, useState } from "react";
import { Alert, Empty, Input, List, Modal, Spin, Tag, message } from "antd";
import { SearchOutlined } from "@ant-design/icons";
import { usePortfolioStore } from "../../stores/portfolioStore";
import type { FundSearchResult } from "../../types";

interface Props {
  open: boolean;
  onClose: () => void;
}

const SEARCH_DEBOUNCE_MS = 400;

export default function AddFundModal({ open, onClose }: Props) {
  const { searchResults, searching, searchError, searchFunds, clearSearch, createFundPortfolio } =
    usePortfolioStore();
  const [keyword, setKeyword] = useState("");
  const [selected, setSelected] = useState<FundSearchResult | null>(null);
  const [creating, setCreating] = useState(false);

  // 输入防抖搜索
  useEffect(() => {
    if (!open) return;
    const trimmed = keyword.trim();
    if (!trimmed) {
      clearSearch();
      return;
    }
    const timer = window.setTimeout(() => {
      searchFunds(trimmed);
    }, SEARCH_DEBOUNCE_MS);
    return () => window.clearTimeout(timer);
  }, [keyword, open, searchFunds, clearSearch]);

  const handleClose = () => {
    setKeyword("");
    setSelected(null);
    clearSearch();
    onClose();
  };

  const handleOk = async () => {
    if (!selected) {
      message.warning("请先从搜索结果中选择一只基金");
      return;
    }
    setCreating(true);
    try {
      await createFundPortfolio(selected);
      message.success(`已添加「${selected.fund_name}」并完成首次刷新`);
      handleClose();
    } catch (err) {
      const text = String(err);
      message.error(text);
      // 组合已创建（仅首刷失败）时关闭弹窗，列表中可稍后手动刷新；
      // 其他失败（如重复添加）保留弹窗让用户改选。
      if (text.includes("组合已创建")) {
        handleClose();
      }
    } finally {
      setCreating(false);
    }
  };

  return (
    <Modal
      title="添加基金"
      open={open}
      onOk={handleOk}
      onCancel={handleClose}
      okText="确认添加"
      cancelText="取消"
      confirmLoading={creating}
      okButtonProps={{ disabled: !selected }}
    >
      <Input
        prefix={<SearchOutlined />}
        placeholder="输入基金名称、代码或拼音，如：兴全 / 163415"
        value={keyword}
        onChange={(e) => {
          setKeyword(e.target.value);
          setSelected(null);
        }}
        allowClear
        autoFocus
      />
      <div style={{ marginTop: 12, maxHeight: 320, overflowY: "auto" }}>
        {searchError && (
          <Alert type="error" showIcon message="搜索失败" description={searchError} />
        )}
        {searching && (
          <div style={{ textAlign: "center", padding: 16 }}>
            <Spin size="small" /> 搜索中…
          </div>
        )}
        {!searching && !searchError && keyword.trim() && searchResults.length === 0 && (
          <Empty image={Empty.PRESENTED_IMAGE_SIMPLE} description="未找到匹配的基金" />
        )}
        {!searching && searchResults.length > 0 && (
          <List
            size="small"
            dataSource={searchResults}
            rowKey={(item) => item.fund_code}
            renderItem={(item) => (
              <List.Item
                onClick={() => setSelected(item)}
                style={{
                  cursor: "pointer",
                  background: selected?.fund_code === item.fund_code ? "#e6f4ff" : undefined,
                  borderRadius: 6,
                  paddingLeft: 8,
                  paddingRight: 8,
                }}
              >
                <span style={{ fontFamily: "monospace", marginRight: 8 }}>{item.fund_code}</span>
                <span style={{ flex: 1 }}>{item.fund_name}</span>
                {item.fund_type && <Tag>{item.fund_type}</Tag>}
              </List.Item>
            )}
          />
        )}
      </div>
    </Modal>
  );
}
