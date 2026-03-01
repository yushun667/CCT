//! MCP Client — 连接外部 MCP 服务器的客户端
//!
//! 通过子进程 stdio 与外部 MCP 服务通信，支持资源浏览和工具调用。
//! 使用 JSON-RPC 2.0 协议进行消息交换。

use std::io::{BufRead, BufReader, Write};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};

use serde::{Deserialize, Serialize};
use tracing::{debug, error, info, warn};

/// MCP 资源描述
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpResource {
    /// 资源 URI（如 `cct://project/info`）
    pub uri: String,
    /// 人类可读名称
    pub name: String,
    /// 资源用途描述
    pub description: String,
}

/// MCP 工具描述
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpTool {
    /// 工具名称
    pub name: String,
    /// 工具用途描述
    pub description: String,
    /// JSON Schema 格式的参数定义
    pub parameters: serde_json::Value,
}

/// MCP Client 实例
///
/// 管理与外部 MCP Server 子进程的通信生命周期。
pub struct McpClient {
    child: Child,
    request_id: AtomicU64,
}

/// JSON-RPC 2.0 请求（客户端发出）
#[derive(Debug, Serialize)]
struct JsonRpcRequest {
    jsonrpc: String,
    id: u64,
    method: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    params: Option<serde_json::Value>,
}

/// JSON-RPC 2.0 响应（从服务端接收）
#[derive(Debug, Deserialize)]
struct JsonRpcResponse {
    #[allow(dead_code)]
    jsonrpc: String,
    #[allow(dead_code)]
    id: serde_json::Value,
    result: Option<serde_json::Value>,
    error: Option<JsonRpcErrorObj>,
}

#[derive(Debug, Deserialize)]
struct JsonRpcErrorObj {
    code: i32,
    message: String,
}

impl McpClient {
    /// 连接外部 MCP Server
    ///
    /// 通过启动子进程并建立 stdio 通道来连接 MCP 服务。
    /// 子进程应实现 MCP 协议的 stdio 传输。
    ///
    /// # 参数
    /// - `command`: 可执行文件路径
    /// - `args`: 命令行参数
    ///
    /// # 返回
    /// 连接成功后返回 `McpClient` 实例
    pub fn connect(command: &str, args: &[&str]) -> Result<Self, Box<dyn std::error::Error>> {
        info!(
            command = command,
            args = ?args,
            "McpClient::connect 连接外部 MCP Server"
        );

        let child = Command::new(command)
            .args(args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|e| {
                error!(command = command, error = %e, "子进程启动失败");
                e
            })?;

        debug!("MCP Server 子进程已启动");

        let mut client = Self {
            child,
            request_id: AtomicU64::new(1),
        };

        client.send_initialize()?;

        Ok(client)
    }

    /// 发送初始化请求
    fn send_initialize(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        info!("McpClient::send_initialize 发送初始化请求");

        let result = self.send_request(
            "initialize",
            Some(serde_json::json!({
                "protocolVersion": "2024-11-05",
                "capabilities": {},
                "clientInfo": {
                    "name": "cct-desktop",
                    "version": env!("CARGO_PKG_VERSION")
                }
            })),
        )?;

        debug!(result = %result, "初始化响应已收到");
        Ok(())
    }

    /// 列出远程 MCP Server 的可用资源
    ///
    /// # 返回
    /// 资源描述列表
    pub fn list_resources(&mut self) -> Result<Vec<McpResource>, Box<dyn std::error::Error>> {
        info!("McpClient::list_resources 查询可用资源");

        let result = self.send_request("resources/list", None)?;

        let resources: Vec<McpResource> = result
            .get("resources")
            .and_then(|v| serde_json::from_value(v.clone()).ok())
            .unwrap_or_default();

        debug!(count = resources.len(), "获取到资源列表");
        Ok(resources)
    }

    /// 读取指定资源的内容
    ///
    /// # 参数
    /// - `uri`: 资源 URI
    ///
    /// # 返回
    /// 资源内容的文本表示
    pub fn read_resource(&mut self, uri: &str) -> Result<String, Box<dyn std::error::Error>> {
        info!(uri = uri, "McpClient::read_resource 读取资源");

        let result = self.send_request(
            "resources/read",
            Some(serde_json::json!({ "uri": uri })),
        )?;

        let text = result
            .get("contents")
            .and_then(|v| v.as_array())
            .and_then(|arr| arr.first())
            .and_then(|c| c.get("text"))
            .and_then(|t| t.as_str())
            .unwrap_or("")
            .to_string();

        debug!(uri = uri, len = text.len(), "资源内容已读取");
        Ok(text)
    }

    /// 列出远程 MCP Server 的可用工具
    ///
    /// # 返回
    /// 工具描述列表
    pub fn list_tools(&mut self) -> Result<Vec<McpTool>, Box<dyn std::error::Error>> {
        info!("McpClient::list_tools 查询可用工具");

        let result = self.send_request("tools/list", None)?;

        let tools: Vec<McpTool> = result
            .get("tools")
            .and_then(|v| serde_json::from_value(v.clone()).ok())
            .unwrap_or_default();

        debug!(count = tools.len(), "获取到工具列表");
        Ok(tools)
    }

    /// 调用远程工具
    ///
    /// # 参数
    /// - `name`: 工具名称
    /// - `params`: 工具参数（JSON 格式）
    ///
    /// # 返回
    /// 工具执行结果（JSON 格式）
    pub fn call_tool(
        &mut self,
        name: &str,
        params: serde_json::Value,
    ) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
        info!(tool = name, "McpClient::call_tool 调用远程工具");

        let result = self.send_request(
            "tools/call",
            Some(serde_json::json!({
                "name": name,
                "arguments": params
            })),
        )?;

        debug!(tool = name, "工具调用完成");
        Ok(result)
    }

    /// 发送 JSON-RPC 请求并等待响应
    fn send_request(
        &mut self,
        method: &str,
        params: Option<serde_json::Value>,
    ) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
        let id = self.request_id.fetch_add(1, Ordering::SeqCst);
        debug!(id = id, method = method, "发送 JSON-RPC 请求");

        let request = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id,
            method: method.to_string(),
            params,
        };

        let stdin = self
            .child
            .stdin
            .as_mut()
            .ok_or("子进程 stdin 不可用")?;
        let request_json = serde_json::to_string(&request)?;
        writeln!(stdin, "{request_json}")?;
        stdin.flush()?;

        let stdout = self
            .child
            .stdout
            .as_mut()
            .ok_or("子进程 stdout 不可用")?;
        let mut reader = BufReader::new(stdout);
        let mut line = String::new();
        reader.read_line(&mut line)?;

        let response: JsonRpcResponse = serde_json::from_str(&line)?;

        if let Some(err) = response.error {
            warn!(
                code = err.code,
                message = %err.message,
                "远程 MCP Server 返回错误"
            );
            return Err(format!("MCP error {}: {}", err.code, err.message).into());
        }

        Ok(response.result.unwrap_or(serde_json::Value::Null))
    }
}

impl Drop for McpClient {
    fn drop(&mut self) {
        info!("McpClient::drop 终止 MCP Server 子进程");
        let _ = self.child.kill();
    }
}
