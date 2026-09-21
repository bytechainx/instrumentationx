# instrumentationx API

本 crate 的公开 API 只有一个 trait。全部条目一经发布即视为稳定面。

## 1. `Instrumentation`

```rust
pub trait Instrumentation: Send + Sync {
    fn record_retry(&self, op: &str, attempt: u32);
    fn record_circuit_open(&self, op: &str);
    fn record_circuit_close(&self, op: &str);
}
```

| 属性 | 值 |
| --- | --- |
| 对象安全 | 是（三个方法都无泛型、无 `Self: Sized` 约束、无关联类型、无 `async fn`） |
| 线程约束 | `Send + Sync` |
| 泛型参数 | 无 |
| 关联类型 | 无 |
| 默认方法体 | 无 |

### 方法语义

| 方法 | 何时调用 | 参数约定 |
| --- | --- | --- |
| `record_retry(op, attempt)` | 消费方决定再试一次时 | `attempt` 的起点（`0` / `1`）由调用方约定，实现方不得假设 |
| `record_circuit_open(op)` | 熔断器由闭合/半开进入打开态时 | 同一次状态迁移只应记录一次 |
| `record_circuit_close(op)` | 熔断器由打开/半开回到闭合态时 | 同一次状态迁移只应记录一次 |

### 实现方义务

1. **必须**满足 `Send + Sync`。
2. **必须**在写入观测后端前清理并限制 `op`：`op` 来自调用方，可能包含控制字符、
   超长字符串或换行，直接写入会造成日志注入或存储放大。
3. **不应**阻塞调用方。慢后端应通过有界缓冲、采样或异步落地在实现方内部消化；
   `record_*` 的耗时会计入消费方的热路径。
4. **不应**在 `record_*` 中 panic。实现方内部错误只能自行内化（计数、丢弃或降级），
   不得向消费方传播。

### 消费方义务

1. 只持有 `&dyn Instrumentation` / `Arc<dyn Instrumentation>`，不依赖任何具体实现类型。
2. `op` 使用稳定的、分层的操作名（如 `db.query`、`exchange.binance.place_order`），
   避免拼接用户输入或高基数字段（订单号、请求 ID 等）——高基数是 metrics 后端的主要成本来源。
3. `attempt` 语义在整个消费方内保持一致。

## 2. 选择指南

| 场景 | 注入什么 |
| --- | --- |
| 生产（`tracing` 落地） | `observex::TracingInstrumentation` |
| 生产（有界进程内 sink） | `observex::ExportingInstrumentation` + 自定义 exporter |
| 单元测试断言调用次数 | `observex::CountingInstrumentation` |
| 不需要任何观测 | 自定义空实现（三个方法体为空），或 `resiliencx::NoopInstrumentation` |

实现方需要在写入前统一清理 `op` 时，应自行实现清理逻辑，不要把清理责任推给消费方。

## 3. 兼容性承诺

- `Instrumentation` 的**方法集合**是稳定面。新增方法属于**破坏性变更**
  （会破坏所有既有实现方），只允许在大版本中引入，且必须提供默认实现体。
- 方法签名（参数类型与次序）视为稳定。
- 本 crate 不含任何具体实现、不引入任何依赖，因此不存在依赖版本兼容矩阵。
