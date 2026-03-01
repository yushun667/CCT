//! MCP (Model Context Protocol) 模块
//!
//! 提供 MCP 协议的服务端与客户端实现，使 CCT 能够：
//! - 作为 MCP Server 向外部 AI 工具暴露项目信息、符号和文件资源
//! - 作为 MCP Client 连接外部 MCP 服务获取扩展能力
//!
//! # 设计说明（桥接模式）
//! 将 MCP 协议层与 CCT 业务逻辑分离：server/client 负责 JSON-RPC 2.0
//! 通信协议，通过 CCT 内部接口访问解析索引和 AI 技能。

pub mod client;
pub mod server;
