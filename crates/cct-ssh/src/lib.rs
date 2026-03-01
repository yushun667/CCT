//! cct-ssh: SSH 通信库
//!
//! 提供 SSH 连接管理、SFTP 文件操作和 Agent RPC 通信功能。
//! M5 阶段实现 — 当前为占位模块，API 形状已定义。

pub mod agent_client;
pub mod connection;
pub mod sftp;

pub use agent_client::AgentRpcClient;
pub use connection::{ConnectionState, SshConnection};
pub use sftp::{RemoteFileEntry, SftpClient};
