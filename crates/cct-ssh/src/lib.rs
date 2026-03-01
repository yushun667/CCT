//! cct-ssh: SSH 通信库
//!
//! 基于 russh 提供 SSH 连接管理、SFTP 文件操作和 Agent RPC 通信功能。
//! 支持密钥认证、密码认证和 SSH Agent 三种认证方式。

pub mod agent_client;
pub mod connection;
pub mod sftp;

pub use agent_client::AgentRpcClient;
pub use connection::{ConnectionState, SshConnection};
pub use sftp::{RemoteFileEntry, SftpClient};
