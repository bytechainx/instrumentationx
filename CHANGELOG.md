# Changelog — instrumentationx

本文件记录 `instrumentationx` 的用户可见变更，遵循 [Keep a Changelog](https://keepachangelog.com/)
与 [Semantic Versioning](https://semver.org/)。

本仓库代码自 `xhyper.rs` 的 `crates/contracts` 抽取而来（抽取时点为 `0.1.6`）。
该工程内的版本线不在本文件中延续，本仓库从 `0.1.0` 重新起算。

## [Unreleased]

## [0.1.0] - 2026-09-21

### 新增

- 首次以独立 crate 形式提供可观测性注入点契约：唯一公开项为对象安全 trait
  `Instrumentation`，含 `record_retry` / `record_circuit_open` / `record_circuit_close` 三个方法。
- 消费方只需持有 `&dyn Instrumentation` / `Arc<dyn Instrumentation>` 即可记录重试与熔断
  状态迁移，无需引入任何具体观测实现。
- 实现方（如 `observex`）只实现该 trait 即可接入消费方（如 `resiliencx`），双方互不依赖，
  不会形成循环依赖或反向业务依赖。

### 变更

- 把原本散落在 `xhyper.rs` 内部 crate `contracts` 中的 `Instrumentation` trait 抽出为独立微
  crate：`[dependencies]` 与 `[dev-dependencies]` 均为空，不再可能间接引入主工程依赖。

### 说明

- **不是** OpenTelemetry API/SDK，不实现 OTLP，也不提供任何具体后端实现。
- 不含错误类型：trait 方法不返回 `Result`，`op` 的清理、限长与非阻塞责任在实现方。
- 本 crate **不发布到 crates.io**，仅以 GitHub 源码 / git 依赖形式复用。
