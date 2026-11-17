# instrumentationx checklist

本文件只复述本仓已有门禁，不新增义务。命令差异以 [README.md](README.md)「门禁」（若有）与 `.github/workflows/ci.yml` 为准。`[x]` 表示本轮已在 crate 根目录跑过且退出码为 0。

来源：工作区 README「质量门禁」默认四条（本仓 README 无「门禁」节）

## P0

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `cargo test`
- [x] `cargo package --no-verify`

## 说明

- 从 crate 根目录运行；执行前核对工作树归属。
- `cargo test` 含 doctest；不要用会漏 doctest 的 `--all-targets` 替代（本仓 README / CI 已写明变体的除外）。
- 不执行 `cargo publish`。README / rustdoc / 元数据不承诺 crates.io 或 docs.rs。
- 工作区级检查在元仓库执行，见工作区 `README.md`「质量门禁」。
- 2026-11-17：规格根 `specs/repos/infra/instrumentationx` Spec Pipeline 收敛包 VERIFIED 后的实现侧对齐复核（HEAD `508497a`）：四条 P0 全过；trait 面与规格根登记一致（`Instrumentation: Send + Sync`，`record_retry` / `record_circuit_open` / `record_circuit_close` 三方法，对象安全，`[dependencies]` / `[dev-dependencies]` 均空）。
- 机器全绿不代替已登记的人审，也不产生合入权。
