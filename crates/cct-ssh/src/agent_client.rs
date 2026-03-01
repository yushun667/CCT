//! Agent RPC 客户端 — 通过 SSH 通道与远程 cct-agent 进行 JSON-RPC 通信
//!
//! 在 SSH 通道上启动远程 cct-agent 进程，通过 stdin/stdout 进行
//! JSON-RPC 2.0 协议通信，支持解析任务管理和索引数据传输。

use cct_core::error::CctError;
use russh::client::Msg;
use russh::{Channel, ChannelMsg};
use serde_json::Value;
use tracing::{debug, error, info, warn};

use crate::connection::SshConnection;

/// Agent JSON-RPC 客户端
///
/// # 设计说明（代理模式 + 命令模式）
/// 作为远程 Agent 的本地代理，将操作请求封装为 JSON-RPC 命令，
/// 通过 SSH 通道发送到远程 cct-agent 进程。
pub struct AgentRpcClient {
    channel: Channel<Msg>,
    request_id: u64,
    host: String,
    buffer: String,
}

impl AgentRpcClient {
    /// 通过 SSH 连接启动远程 Agent 并建立 RPC 通道
    ///
    /// # 参数
    /// - `conn`: 已认证的 SSH 连接
    /// - `agent_bin`: 远程 Agent 二进制文件路径
    ///
    /// # 返回
    /// - `Ok(Self)`: RPC 通道就绪
    /// - `Err(CctError)`: Agent 启动失败
    pub async fn from_connection(
        conn: &SshConnection,
        agent_bin: &str,
    ) -> Result<Self, CctError> {
        info!(
            host = %conn.config().host,
            agent_bin = %agent_bin,
            "AgentRpcClient::from_connection — 启动远程 Agent"
        );

        let channel = conn.open_channel().await?;

        debug!(agent_bin = %agent_bin, "在 SSH 通道上执行 Agent 二进制...");
        channel.exec(true, agent_bin).await.map_err(|e| {
            error!(agent_bin = %agent_bin, error = %e, "Agent 启动失败");
            CctError::AgentStartFailed(format!("启动 Agent 失败: {e}"))
        })?;

        info!(host = %conn.config().host, "Agent 已启动，RPC 通道就绪");

        Ok(Self {
            channel,
            request_id: 0,
            host: conn.config().host.clone(),
            buffer: String::new(),
        })
    }

    /// 发送 JSON-RPC 请求并等待响应
    ///
    /// # 参数
    /// - `method`: RPC 方法名（如 `parse/start`、`agent/version`）
    /// - `params`: 请求参数（JSON Value）
    ///
    /// # 返回
    /// - `Ok(Value)`: 响应的 `result` 字段
    /// - `Err(CctError)`: 通信或协议错误
    pub async fn send_request(
        &mut self,
        method: &str,
        params: Value,
    ) -> Result<Value, CctError> {
        self.request_id += 1;
        let id = self.request_id;

        info!(
            host = %self.host, method = %method, request_id = id,
            "AgentRpcClient::send_request — 发送 RPC 请求"
        );

        let request = serde_json::json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": method,
            "params": params,
        });

        let mut request_str = serde_json::to_string(&request).map_err(|e| {
            CctError::Internal(format!("请求序列化失败: {e}"))
        })?;
        request_str.push('\n');

        debug!(method = %method, "发送请求数据...");
        self.channel
            .data(request_str.as_bytes())
            .await
            .map_err(|e| {
                error!(method = %method, error = %e, "通道数据发送失败");
                CctError::RemoteAgentUnreachable
            })?;

        self.read_response().await
    }

    /// 从通道读取一行 JSON-RPC 响应
    async fn read_response(&mut self) -> Result<Value, CctError> {
        debug!(host = %self.host, "等待 Agent 响应...");

        loop {
            if let Some(pos) = self.buffer.find('\n') {
                let line = self.buffer[..pos].to_string();
                self.buffer = self.buffer[pos + 1..].to_string();

                debug!(response_len = line.len(), "收到完整响应行");
                return self.parse_rpc_response(&line);
            }

            let Some(msg) = self.channel.wait().await else {
                error!("SSH 通道已关闭，Agent 不可达");
                return Err(CctError::RemoteAgentUnreachable);
            };

            match msg {
                ChannelMsg::Data { ref data } => {
                    let text = String::from_utf8_lossy(data);
                    self.buffer.push_str(&text);
                }
                ChannelMsg::ExitStatus { exit_status } => {
                    warn!(exit_status, "Agent 进程已退出");
                    if exit_status != 0 {
                        return Err(CctError::RemoteAgentParseFailed(format!(
                            "Agent 异常退出，退出码: {exit_status}"
                        )));
                    }
                }
                ChannelMsg::Eof => {
                    debug!("收到 EOF，Agent 输出结束");
                    if !self.buffer.trim().is_empty() {
                        let line = std::mem::take(&mut self.buffer);
                        return self.parse_rpc_response(line.trim());
                    }
                    return Err(CctError::RemoteAgentUnreachable);
                }
                _ => {}
            }
        }
    }

    /// 解析 JSON-RPC 2.0 响应，提取 result 或转换 error
    fn parse_rpc_response(&self, line: &str) -> Result<Value, CctError> {
        debug!(host = %self.host, "解析 RPC 响应");

        let response: Value = serde_json::from_str(line).map_err(|e| {
            error!(raw = %line, error = %e, "RPC 响应 JSON 解析失败");
            CctError::Internal(format!("RPC 响应解析失败: {e}"))
        })?;

        if let Some(error) = response.get("error") {
            let message = error
                .get("message")
                .and_then(|m| m.as_str())
                .unwrap_or("未知 RPC 错误");
            let code = error.get("code").and_then(|c| c.as_i64()).unwrap_or(-1);
            warn!(code, message, "Agent 返回 RPC 错误");
            return Err(CctError::RemoteAgentParseFailed(format!(
                "RPC 错误 [{code}]: {message}"
            )));
        }

        Ok(response
            .get("result")
            .cloned()
            .unwrap_or(Value::Null))
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

    /// 获取 Agent 版本信息
    pub async fn get_version(&mut self) -> Result<Value, CctError> {
        info!("AgentRpcClient::get_version — 查询 Agent 版本");
        self.send_request("agent/version", Value::Null).await
    }

    /// 请求 Agent 安全退出
    pub async fn shutdown(&mut self) -> Result<Value, CctError> {
        info!("AgentRpcClient::shutdown — 请求 Agent 退出");
        self.send_request("agent/shutdown", Value::Null).await
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
        self.send_request(
            "index/transfer",
            serde_json::json!({ "path": remote_db_path }),
        )
        .await
    }
}
