//! Agent RPC 客户端 — 通过 SSH 通道与远程 Agent 通信
//!
//! 封装 JSON-RPC 协议，将解析请求转发到远程 cct-agent 进程。
//! 当前为占位实现，不执行真实网络通信。

use cct_core::error::CctError;
use serde_json::Value;
use tracing::{debug, info, warn};

/// Agent JSON-RPC 客户端
///
/// # 设计说明（代理模式 + 命令模式）
/// 作为远程 Agent 的本地代理，将操作请求封装为 JSON-RPC 命令发送。
/// 上层调用者无需关心传输层细节。
pub struct AgentRpcClient {
    host: String,
    connected: bool,
    request_id: u64,
}

impl AgentRpcClient {
    /// 创建 RPC 客户端实例
    ///
    /// # 参数
    /// - `host`: 远程主机标识
    pub fn new(host: String) -> Self {
        info!(host = %host, "AgentRpcClient::new — 创建 Agent RPC 客户端");
        Self {
            host,
            connected: true,
            request_id: 0,
        }
    }

    /// 发送 JSON-RPC 请求
    ///
    /// # 参数
    /// - `method`: RPC 方法名（如 `parse/start`）
    /// - `params`: 请求参数
    ///
    /// # 返回
    /// - `Ok(Value)`: 响应结果
    /// - `Err(CctError)`: 通信失败
    pub async fn send_request(
        &mut self,
        method: &str,
        params: Value,
    ) -> Result<Value, CctError> {
        self.request_id += 1;
        let id = self.request_id;

        info!(
            host = %self.host,
            method = %method,
            request_id = id,
            "AgentRpcClient::send_request — 发送 RPC 请求"
        );

        if !self.connected {
            return Err(CctError::RemoteAgentUnreachable);
        }

        debug!(
            method = %method,
            params = %params,
            "占位实现：模拟 RPC 响应"
        );

        // 占位实现：根据方法名返回模拟数据
        let result = match method {
            "agent/version" => serde_json::json!({
                "version": env!("CARGO_PKG_VERSION"),
                "platform": std::env::consts::OS,
                "arch": std::env::consts::ARCH,
            }),
            "parse/status" => serde_json::json!({
                "status": "idle",
                "progress": null,
            }),
            _ => serde_json::json!({
                "ok": true,
                "message": format!("占位响应: {}", method),
            }),
        };

        Ok(serde_json::json!({
            "jsonrpc": "2.0",
            "id": id,
            "result": result,
        }))
    }

    /// 启动远程解析
    ///
    /// # 参数
    /// - `source_root`: 远程源码根目录
    /// - `compile_db_path`: 编译数据库路径（可选）
    pub async fn start_parse(
        &mut self,
        source_root: &str,
        compile_db_path: Option<&str>,
    ) -> Result<Value, CctError> {
        info!(
            source_root = %source_root,
            "AgentRpcClient::start_parse — 请求远程解析"
        );
        self.send_request(
            "parse/start",
            serde_json::json!({
                "source_root": source_root,
                "compile_db_path": compile_db_path,
            }),
        )
        .await
    }

    /// 取消远程解析
    pub async fn cancel_parse(&mut self) -> Result<Value, CctError> {
        info!("AgentRpcClient::cancel_parse — 取消远程解析");
        self.send_request("parse/cancel", Value::Null).await
    }

    /// 获取远程解析状态
    pub async fn get_status(&mut self) -> Result<Value, CctError> {
        info!("AgentRpcClient::get_status — 查询远程状态");
        self.send_request("parse/status", Value::Null).await
    }

    /// 传输索引数据到本地
    ///
    /// # 参数
    /// - `remote_db_path`: 远程索引数据库路径
    pub async fn transfer_index(
        &mut self,
        remote_db_path: &str,
    ) -> Result<Value, CctError> {
        info!(
            remote_db_path = %remote_db_path,
            "AgentRpcClient::transfer_index — 传输远程索引"
        );
        warn!("transfer_index 为占位实现");
        self.send_request(
            "index/transfer",
            serde_json::json!({ "path": remote_db_path }),
        )
        .await
    }
}
