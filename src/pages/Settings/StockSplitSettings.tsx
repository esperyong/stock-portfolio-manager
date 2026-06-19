import { useEffect, useState } from "react";
import {
  Card,
  Table,
  Button,
  Modal,
  Form,
  Input,
  InputNumber,
  DatePicker,
  Space,
  Popconfirm,
  message,
  Typography,
} from "antd";
import { PlusOutlined, DeleteOutlined } from "@ant-design/icons";
import { useStockSplitStore } from "../../stores/stockSplitStore";
import type { StockSplit } from "../../types";
import type { Dayjs } from "dayjs";

const { Text } = Typography;

export default function StockSplitSettings() {
  const { splits, loading, fetchSplits, addSplit, deleteSplit } = useStockSplitStore();
  const [modalOpen, setModalOpen] = useState(false);
  const [form] = Form.useForm();
  const [submitting, setSubmitting] = useState(false);

  useEffect(() => {
    fetchSplits();
  }, [fetchSplits]);

  const handleAdd = async () => {
    try {
      const values = await form.validateFields();
      const splitDate: Dayjs = values.split_date;
      setSubmitting(true);
      await addSplit(
        values.stock_code,
        splitDate.format("YYYY-MM-DD"),
        values.ratio_from,
        values.ratio_to
      );
      message.success("拆股信息已添加");
      form.resetFields();
      setModalOpen(false);
    } catch (err) {
      if (err && typeof err === "object" && "errorFields" in err) return; // form validation error
      message.error(`添加失败: ${err}`);
    } finally {
      setSubmitting(false);
    }
  };

  const handleDelete = async (id: number) => {
    try {
      await deleteSplit(id);
      message.success("已删除");
    } catch (err) {
      message.error(`删除失败: ${err}`);
    }
  };

  const columns = [
    {
      title: "股票代码",
      dataIndex: "stock_code",
      key: "stock_code",
      width: 120,
    },
    {
      title: "拆股日期",
      dataIndex: "split_date",
      key: "split_date",
      width: 140,
    },
    {
      title: "拆股比例",
      key: "ratio",
      width: 140,
      render: (_: unknown, record: StockSplit) => (
        <Text>
          {record.ratio_from} → {record.ratio_to}
        </Text>
      ),
    },
    {
      title: "操作",
      key: "actions",
      width: 80,
      render: (_: unknown, record: StockSplit) => (
        <Popconfirm
          title="确定删除此拆股信息？"
          onConfirm={() => handleDelete(record.id)}
          okText="删除"
          cancelText="取消"
        >
          <Button type="text" danger icon={<DeleteOutlined />} size="small" />
        </Popconfirm>
      ),
    },
  ];

  return (
    <div className="space-y-6">
      <Card
        title="拆股管理"
        extra={
          <Button type="primary" icon={<PlusOutlined />} onClick={() => setModalOpen(true)}>
            添加拆股
          </Button>
        }
      >
        <Text type="secondary" style={{ display: "block", marginBottom: 16 }}>
          拆股信息用于期权合约匹配。当股票在期权合约存续期间发生拆股，系统会根据拆股比例自动匹配调整后的合约。
          <br />
          例如：比亚迪(1211)在 2025-06-10 进行 1拆3，则填写股票代码 1211，日期 2025-06-10，比例 1 → 3。
        </Text>
        <Table
          dataSource={splits}
          columns={columns}
          rowKey="id"
          loading={loading}
          size="small"
          pagination={false}
          locale={{ emptyText: "暂无拆股信息" }}
        />
      </Card>

      <Modal
        title="添加拆股信息"
        open={modalOpen}
        onOk={handleAdd}
        onCancel={() => {
          form.resetFields();
          setModalOpen(false);
        }}
        confirmLoading={submitting}
        okText="添加"
        cancelText="取消"
      >
        <Form form={form} layout="vertical" style={{ marginTop: 16 }}>
          <Form.Item
            label="股票代码"
            name="stock_code"
            rules={[{ required: true, message: "请输入股票代码" }]}
          >
            <Input placeholder="例如：1211" />
          </Form.Item>
          <Form.Item
            label="拆股日期"
            name="split_date"
            rules={[{ required: true, message: "请选择拆股日期" }]}
          >
            <DatePicker style={{ width: "100%" }} />
          </Form.Item>
          <Space style={{ width: "100%" }}>
            <Form.Item
              label="拆股前"
              name="ratio_from"
              rules={[{ required: true, message: "必填" }]}
              initialValue={1}
            >
              <InputNumber min={1} max={100} style={{ width: 120 }} />
            </Form.Item>
            <Text style={{ marginTop: 30 }}>→</Text>
            <Form.Item
              label="拆股后"
              name="ratio_to"
              rules={[{ required: true, message: "必填" }]}
            >
              <InputNumber min={1} max={100} style={{ width: 120 }} placeholder="例如：3" />
            </Form.Item>
          </Space>
          <Text type="secondary">
            比例示例：1拆3 则左边填 1，右边填 3；2拆1（合股）则左边填 2，右边填 1。
          </Text>
        </Form>
      </Modal>
    </div>
  );
}
