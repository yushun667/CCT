//! MCP Server — 基于 stdio 的 JSON-RPC 2.0 服务端
//!
//! 将 CCT 的项目信息、符号索引和文件内容以 MCP 资源形式暴露给外部 AI 工具，
//! 同时将 CCT 内置 AI 技能映射为 MCP Tools 供外部调用。
//!
//! # 资源（Resources）
//! - `cct://project/info` — 当前项目的基本信息
//! - `cct://symbols/{id}` — 按 ID 查询符号详情
//! - `cct://file/{path}` — 读取指定文件内容
//!
//! # 工具（Tools）
//! 映射到现有 AI 技能（搜索符号、查询调用链等）

use std::io::{self, BufRead, Write};

use serde::{Deserialize, Serialize};
use tracing::{debug, error, info, warn};

/// JSON-RPC 2.0 请求
#[derive(Debug, Deserialize)]
struct JsonRpcRequest {
    jsonrpc: String,
    id: Option<serde_json::Value>,
    method: String,
    #[serde(default)]
    params: serde_json::Value,
}

/// JSON-RPC 2.0 响应
#[derive(Debug, Serialize)]
struct JsonRpcResponse {
    jsonrpc: String,
    id: serde_json::Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    result: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<JsonRpcError>,
}

/// JSON-RPC 2.0 错误
#[derive(Debug, Serialize)]
struct JsonRpcError {
    code: i32,
    message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    data: Option<serde_json::Value>,
}

/// MCP 资源描述
#[derive(Debug, Clone, Serialize)]
struct McpResourceDescriptor {
    uri: String,
    name: String,
    description: String,
    #[serde(rename = "mimeType")]
    mime_type: String,
}

/// MCP 工具描述
#[derive(Debug, Clone, Serialize)]
struct McpToolDescriptor {
    name: String,
    description: String,
    #[serde(rename = "inputSchema")]
    input_schema: serde_json::Value,
}

/// MCP Server 实例
///
/// 通过 stdio（标准输入/输出）与外部 AI 客户端通信，
/// 实现 MCP 协议的资源读取和工具调用。
pub struct McpServer {
    /// 服务器名称
    name: String,
    /// 服务器版本
    version: String,
    /// 是否正在运行
    running: bool,
}

impl McpServer {
    /// 创建 MCP Server 实例
    ///
    /// # 参数
    /// - `name`: 服务器标识名称
    /// - `version`: 版本号
    pub fn new(name: &str, version: &str) -> Self {
        info!(
            name = name,
            version = version,
            "McpServer::new 创建 MCP Server"
        );
        Self {
            name: name.to_string(),
            version: version.to_string(),
            running: false,
        }
    }

    /// 启动 MCP Server — 循环读取 stdin 并分发处理
    ///
    /// 每行读取一个 JSON-RPC 2.0 请求，处理后将响应写入 stdout。
    /// 调用 `shutdown` 方法或收到 EOF 时退出。
    pub fn start(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        info!("McpServer::start 启动 MCP stdio 服务");
        self.running = true;

        let stdin = io::stdin();
        let mut stdout = io::stdout();

        for line in stdin.lock().lines() {
            if !self.running {
                info!("MCP Server 已停止");
                break;
            }

            let line = match line {
                Ok(l) => l,
                Err(e) => {
                    error!(error = %e, "读取 stdin 失败");
                    break;
                }
            };

            if line.trim().is_empty() {
                continue;
            }

            debug!(raw = %line, "收到 JSON-RPC 请求");

            let response = match serde_json::from_str::<JsonRpcRequest>(&line) {
                Ok(req) => self.dispatch(req),
                Err(e) => {
                    warn!(error = %e, "JSON-RPC 解析失败");
                    JsonRpcResponse {
                        jsonrpc: "2.0".to_string(),
                        id: serde_json::Value::Null,
                        result: None,
                        error: Some(JsonRpcError {
                            code: -32700,
                            message: format!("Parse error: {e}"),
                            data: None,
                        }),
                    }
                }
            };

            let resp_json = serde_json::to_string(&response)?;
            writeln!(stdout, "{resp_json}")?;
            stdout.flush()?;

            debug!("响应已发送");
        }

        info!("McpServer::start MCP Server 主循环结束");
        Ok(())
    }

    /// 停止 MCP Server
    pub fn shutdown(&mut self) {
        info!("McpServer::shutdown 关闭 MCP Server");
        self.running = false;
    }

    /// 请求分发 — 根据 method 路由到对应处理器
    fn dispatch(&self, req: JsonRpcRequest) -> JsonRpcResponse {
        let id = req.id.clone().unwrap_or(serde_json::Value::Null);
        info!(method = %req.method, "McpServer::dispatch 分发请求");

        let result = match req.method.as_str() {
            "initialize" => self.handle_initialize(&req.params),
            "resources/list" => self.handle_list_resources(),
            "resources/read" => self.handle_read_resource(&req.params),
            "tools/list" => self.handle_list_tools(),
            "tools/call" => self.handle_call_tool(&req.params),
            _ => {
                warn!(method = %req.method, "未知方法");
                Err(JsonRpcError {
                    code: -32601,
                    message: format!("Method not found: {}", req.method),
                    data: None,
                })
            }
        };

        match result {
            Ok(value) => JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                id,
                result: Some(value),
                error: None,
            },
            Err(err) => JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                id,
                result: None,
                error: Some(err),
            },
        }
    }

    /// 处理 initialize — 返回服务器能力声明
    fn handle_initialize(
        &self,
        _params: &serde_json::Value,
    ) -> Result<serde_json::Value, JsonRpcError> {
        info!("McpServer::handle_initialize 处理初始化请求");

        Ok(serde_json::json!({
            "protocolVersion": "2024-11-05",
            "capabilities": {
                "resources": { "subscribe": false, "listChanged": false },
                "tools": {}
            },
            "serverInfo": {
                "name": self.name,
                "version": self.version
            }
        }))
    }

    /// 列出所有可用资源
    fn handle_list_resources(&self) -> Result<serde_json::Value, JsonRpcError> {
        info!("McpServer::handle_list_resources 列出可用资源");

        let resources = vec![
            McpResourceDescriptor {
                uri: "cct://project/info".to_string(),
                name: "Project Info".to_string(),
                description: "当前 CCT 项目的基本信息（名称、路径、解析状态等）".to_string(),
                mime_type: "application/json".to_string(),
            },
            McpResourceDescriptor {
                uri: "cct://symbols/{id}".to_string(),
                name: "Symbol Details".to_string(),
                description: "按 ID 查询符号详情（名称、类型、位置等）".to_string(),
                mime_type: "application/json".to_string(),
            },
            McpResourceDescriptor {
                uri: "cct://file/{path}".to_string(),
                name: "File Content".to_string(),
                description: "读取指定源文件的内容".to_string(),
                mime_type: "text/plain".to_string(),
            },
        ];

        Ok(serde_json::json!({ "resources": resources }))
    }

    /// 读取指定资源
    fn handle_read_resource(
        &self,
        params: &serde_json::Value,
    ) -> Result<serde_json::Value, JsonRpcError> {
        let uri = params
            .get("uri")
            .and_then(|v| v.as_str())
            .unwrap_or_default();

        info!(uri = uri, "McpServer::handle_read_resource 读取资源");

        // placeholder: 返回占位数据
        let content = match uri {
            "cct://project/info" => {
                debug!("返回项目信息占位数据");
                serde_json::json!({
                    "name": "CCT Project",
                    "status": "placeholder",
                    "message": "MCP Server 资源读取功能待与索引数据库集成"
                })
            }
            _ if uri.starts_with("cct://symbols/") => {
                let symbol_id = uri.strip_prefix("cct://symbols/").unwrap_or("0");
                debug!(symbol_id = symbol_id, "返回符号占位数据");
                serde_json::json!({
                    "id": symbol_id,
                    "status": "placeholder",
                    "message": "符号查询待集成"
                })
            }
            _ if uri.starts_with("cct://file/") => {
                let file_path = uri.strip_prefix("cct://file/").unwrap_or("");
                debug!(file_path = file_path, "返回文件内容占位数据");
                serde_json::json!({
                    "path": file_path,
                    "status": "placeholder",
                    "message": "文件读取待集成"
                })
            }
            _ => {
                warn!(uri = uri, "未知资源 URI");
                return Err(JsonRpcError {
                    code: -32602,
                    message: format!("Unknown resource URI: {uri}"),
                    data: None,
                });
            }
        };

        Ok(serde_json::json!({
            "contents": [{
                "uri": uri,
                "text": serde_json::to_string_pretty(&content).unwrap_or_default()
            }]
        }))
    }

    /// 列出所有可用工具
    fn handle_list_tools(&self) -> Result<serde_json::Value, JsonRpcError> {
        info!("McpServer::handle_list_tools 列出可用工具");

        let tools = vec![
            McpToolDescriptor {
                name: "search_symbols".to_string(),
                description: "在索引中搜索符号（函数、变量、类型、宏）".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "query": { "type": "string", "description": "搜索关键词" },
                        "kind": { "type": "string", "description": "符号类型过滤", "enum": ["function", "variable", "type", "macro"] }
                    },
                    "required": ["query"]
                }),
            },
            McpToolDescriptor {
                name: "query_call_graph".to_string(),
                description: "查询函数的调用图（调用者和被调用者）".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "symbol_name": { "type": "string", "description": "函数名" },
                        "direction": { "type": "string", "enum": ["callers", "callees", "both"] },
                        "depth": { "type": "integer", "description": "查询深度", "default": 3 }
                    },
                    "required": ["symbol_name"]
                }),
            },
            McpToolDescriptor {
                name: "analyze_file".to_string(),
                description: "分析指定文件的符号和依赖关系".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "file_path": { "type": "string", "description": "文件路径" }
                    },
                    "required": ["file_path"]
                }),
            },
        ];

        Ok(serde_json::json!({ "tools": tools }))
    }

    /// 调用指定工具
    fn handle_call_tool(
        &self,
        params: &serde_json::Value,
    ) -> Result<serde_json::Value, JsonRpcError> {
        let tool_name = params
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or_default();
        let arguments = params
            .get("arguments")
            .cloned()
            .unwrap_or(serde_json::Value::Object(Default::default()));

        info!(
            tool = tool_name,
            "McpServer::handle_call_tool 调用工具"
        );

        // placeholder: 返回占位结果
        let result_text = match tool_name {
            "search_symbols" => {
                let query = arguments
                    .get("query")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                debug!(query = query, "执行符号搜索（占位）");
                format!("符号搜索 '{query}' — 功能待与索引数据库集成")
            }
            "query_call_graph" => {
                let symbol = arguments
                    .get("symbol_name")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                debug!(symbol = symbol, "执行调用图查询（占位）");
                format!("调用图查询 '{symbol}' — 功能待与查询引擎集成")
            }
            "analyze_file" => {
                let path = arguments
                    .get("file_path")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                debug!(path = path, "执行文件分析（占位）");
                format!("文件分析 '{path}' — 功能待集成")
            }
            _ => {
                warn!(tool = tool_name, "未知工具");
                return Err(JsonRpcError {
                    code: -32602,
                    message: format!("Unknown tool: {tool_name}"),
                    data: None,
                });
            }
        };

        Ok(serde_json::json!({
            "content": [{
                "type": "text",
                "text": result_text
            }]
        }))
    }
}
