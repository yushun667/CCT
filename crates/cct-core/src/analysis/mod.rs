/// 专项分析模块 — 针对 Linux 内核、OpenHarmony 和自定义规则的深度分析
///
/// # 模块职责
/// 提供面向特定 C/C++ 生态的高级分析能力：
/// - `linux_kernel`: Linux 内核系统调用与 ioctl 分析
/// - `openharmony`: OpenHarmony IPC 服务发现与通信追踪
/// - `custom_rules`: 基于 YAML 的自定义规则引擎
///
/// # 设计说明（策略模式）
/// 各分析器独立实现，通过统一接口对外暴露，
/// 使用者可按需选择分析策略。

pub mod custom_rules;
pub mod linux_kernel;
pub mod openharmony;

pub use custom_rules::{CustomRule, RuleEngine, RuleMatch, RulePattern, Severity};
pub use linux_kernel::{IoctlCommand, LinuxKernelAnalyzer, SyscallInfo};
pub use openharmony::{IpcService, MessageCode, OpenHarmonyAnalyzer};
