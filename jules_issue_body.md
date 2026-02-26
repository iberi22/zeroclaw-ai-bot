## Task Description
Jules, please conduct a comprehensive scan of this repository to identify and implement architectural improvements and modernize dependencies.

### Key Objectives
1.  **Architecture Review**: Analyze the current trait-driven modular architecture (`src/providers`, `src/channels`, `src/tools`, etc.) and ensure it follows the protocols defined in `AGENTS.md` and `CLAUDE.md`.
2.  **Dependency Update**: Scan `Cargo.toml` and update libraries to their latest stable versions. Leverage new features offered by these updates where applicable (e.g., improved async patterns in `tokio`, enhanced serialization in `serde`).
3.  **Resilience Validation**: Verify the recently implemented resilience module (`src/resilience.rs`) and `ResilienceObserver`. Ensure crash detection and task reporting are fully functional and robust.
4.  **Proactivity Enhancement**: Review `SOUL.md` and `IDENTITY.md`. Ensure the agent's behavior aligns with the "senior engineer" persona and proactive exploration directives.
5.  **Build & Test**: Perform full verifications (`cargo check`, `cargo test`) after each set of changes.

### Context
- The repository is a Rust-first autonomous agent runtime.
- Security and performance are critical product goals.
- Use the provided project instructions and protocol files as your primary guidance.

### Reference
- [Getting Started](docs/getting-started/README.md)
- [Architecture Hub](docs/README.md)
