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

use cct_core::indexer::database::IndexDatabase;
use cct_core::query::{CallQueryEngine, IncludeQueryEngine, SymbolSearchEngine};

use crate::ai::skills::index_db_path;

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
    /// 当前活动的项目 ID，用于定位索引数据库
    current_project_id: Option<String>,
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
            current_project_id: None,
        }
    }

    /// 设置当前项目 ID
    ///
    /// MCP 工具和资源查询需要知道目标项目，
    /// 在启动前或处理 initialize 请求时调用。
    pub fn set_project_id(&mut self, project_id: &str) {
        info!(project_id = %project_id, "McpServer::set_project_id 设置当前项目");
        self.current_project_id = Some(project_id.to_string());
    }

    /// 尝试打开当前项目的索引数据库
    fn open_db(&self) -> Result<IndexDatabase, JsonRpcError> {
        let project_id = self.current_project_id.as_deref().ok_or_else(|| {
            JsonRpcError {
                code: -32603,
                message: "未设置当前项目 ID".to_string(),
                data: None,
            }
        })?;

        let db_path = index_db_path(project_id);
        IndexDatabase::open(&db_path).map_err(|e| JsonRpcError {
            code: -32603,
            message: format!("无法打开索引数据库: {}", e),
            data: None,
        })
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
    fn dispatch(&mut self, req: JsonRpcRequest) -> JsonRpcResponse {
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

    /// 处理 initialize — 返回服务器能力声明，并提取 project_id
    fn handle_initialize(
        &mut self,
        params: &serde_json::Value,
    ) -> Result<serde_json::Value, JsonRpcError> {
        info!("McpServer::handle_initialize 处理初始化请求");

        if let Some(pid) = params
            .get("initializationOptions")
            .and_then(|o| o.get("project_id"))
            .and_then(|v| v.as_str())
        {
            self.set_project_id(pid);
        }

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

    /// 读取指定资源 — 连接索引数据库返回真实数据
    fn handle_read_resource(
        &self,
        params: &serde_json::Value,
    ) -> Result<serde_json::Value, JsonRpcError> {
        let uri = params
            .get("uri")
            .and_then(|v| v.as_str())
            .unwrap_or_default();

        info!(uri = uri, "McpServer::handle_read_resource 读取资源");

        let content = match uri {
            "cct://project/info" => {
                debug!("返回项目信息");
                self.read_project_info()?
            }
            _ if uri.starts_with("cct://symbols/") => {
                let symbol_id_str = uri.strip_prefix("cct://symbols/").unwrap_or("0");
                debug!(symbol_id = symbol_id_str, "查询符号详情");
                self.read_symbol_details(symbol_id_str)?
            }
            _ if uri.starts_with("cct://file/") => {
                let file_path = uri.strip_prefix("cct://file/").unwrap_or("");
                debug!(file_path = file_path, "读取文件内容");
                self.read_file_content(file_path)?
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

    fn read_project_info(&self) -> Result<serde_json::Value, JsonRpcError> {
        info!("McpServer::read_project_info 读取项目信息");
        let project_id = self.current_project_id.as_deref().unwrap_or("unknown");

        match self.open_db() {
            Ok(db) => {
                let stats = db.get_statistics().map_err(|e| JsonRpcError {
                    code: -32603,
                    message: format!("统计信息查询失败: {}", e),
                    data: None,
                })?;

                Ok(serde_json::json!({
                    "project_id": project_id,
                    "status": "indexed",
                    "statistics": {
                        "total_files": stats.total_files,
                        "parsed_files": stats.parsed_files,
                        "total_symbols": stats.total_symbols,
                        "total_functions": stats.total_functions,
                        "total_variables": stats.total_variables,
                        "total_types": stats.total_types,
                        "total_macros": stats.total_macros,
                        "total_call_relations": stats.total_call_relations,
                        "total_include_relations": stats.total_include_relations,
                    }
                }))
            }
            Err(_) => {
                Ok(serde_json::json!({
                    "project_id": project_id,
                    "status": "not_indexed",
                    "message": "索引数据库未找到或无法打开"
                }))
            }
        }
    }

    fn read_symbol_details(&self, symbol_id_str: &str) -> Result<serde_json::Value, JsonRpcError> {
        info!(symbol_id = %symbol_id_str, "McpServer::read_symbol_details 查询符号");
        let id: i64 = symbol_id_str.parse().map_err(|_| JsonRpcError {
            code: -32602,
            message: format!("无效的符号 ID: {}", symbol_id_str),
            data: None,
        })?;

        let db = self.open_db()?;
        let symbol = db.lookup_symbol(id).ok_or_else(|| JsonRpcError {
            code: -32602,
            message: format!("符号 ID={} 未找到", id),
            data: None,
        })?;

        Ok(serde_json::json!({
            "id": symbol.id,
            "name": symbol.name,
            "qualified_name": symbol.qualified_name,
            "kind": format!("{}", symbol.kind),
            "file_path": symbol.file_path,
            "line": symbol.line,
            "column": symbol.column,
            "end_line": symbol.end_line,
            "is_definition": symbol.is_definition,
            "return_type": symbol.return_type,
            "parameters": symbol.parameters,
            "access": symbol.access.as_ref().map(|a| format!("{:?}", a)),
        }))
    }

    fn read_file_content(&self, file_path: &str) -> Result<serde_json::Value, JsonRpcError> {
        info!(file = %file_path, "McpServer::read_file_content 读取文件");
        let content = std::fs::read_to_string(file_path).map_err(|e| JsonRpcError {
            code: -32603,
            message: format!("文件读取失败 {}: {}", file_path, e),
            data: None,
        })?;

        Ok(serde_json::json!({
            "path": file_path,
            "content": content,
            "lines": content.lines().count(),
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

    /// 调用指定工具 — 连接索引数据库执行真实查询
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

        let result_text = match tool_name {
            "search_symbols" => self.tool_search_symbols(&arguments)?,
            "query_call_graph" => self.tool_query_call_graph(&arguments)?,
            "analyze_file" => self.tool_analyze_file(&arguments)?,
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

    fn tool_search_symbols(&self, args: &serde_json::Value) -> Result<String, JsonRpcError> {
        info!("McpServer::tool_search_symbols 搜索符号");
        let query = args.get("query").and_then(|v| v.as_str()).unwrap_or("");
        let kind_filter = args.get("kind").and_then(|v| v.as_str());

        let db = self.open_db()?;

        let symbols = if let Some(kind_str) = kind_filter {
            let kind = cct_core::indexer::database::str_to_symbol_kind(kind_str);
            SymbolSearchEngine::search_by_kind(&db, query, kind, 50)
        } else {
            SymbolSearchEngine::search(&db, query, 50)
        }
        .map_err(|e| JsonRpcError {
            code: -32603,
            message: format!("搜索失败: {}", e),
            data: None,
        })?;

        if symbols.is_empty() {
            return Ok(format!("未找到匹配 '{}' 的符号。", query));
        }

        let mut lines = vec![format!("找到 {} 个匹配 '{}' 的符号:\n", symbols.len(), query)];
        for s in &symbols {
            lines.push(format!(
                "- [{}] `{}` @ {}:{} (ID={})",
                s.kind, s.qualified_name, s.file_path, s.line, s.id
            ));
        }

        Ok(lines.join("\n"))
    }

    fn tool_query_call_graph(&self, args: &serde_json::Value) -> Result<String, JsonRpcError> {
        info!("McpServer::tool_query_call_graph 查询调用图");
        let symbol_name = args.get("symbol_name").and_then(|v| v.as_str()).unwrap_or("");
        let direction = args.get("direction").and_then(|v| v.as_str()).unwrap_or("both");
        let depth = args.get("depth").and_then(|v| v.as_u64()).unwrap_or(3) as u32;

        let db = self.open_db()?;

        let symbols = SymbolSearchEngine::search(&db, symbol_name, 5).map_err(|e| JsonRpcError {
            code: -32603,
            message: format!("符号搜索失败: {}", e),
            data: None,
        })?;

        let target = symbols.first().ok_or_else(|| JsonRpcError {
            code: -32602,
            message: format!("未找到符号 '{}'", symbol_name),
            data: None,
        })?;

        let mut lines = vec![format!(
            "## 调用图: `{}` (ID={})\n",
            target.qualified_name, target.id
        )];

        if direction == "callers" || direction == "both" {
            let callers = CallQueryEngine::query_callers(&db, target.id, depth).map_err(|e| {
                JsonRpcError {
                    code: -32603,
                    message: format!("调用者查询失败: {}", e),
                    data: None,
                }
            })?;
            lines.push(format!("### 调用者（共 {} 处）\n", callers.len()));
            for r in &callers {
                let name = db.lookup_symbol_name(r.caller_id).unwrap_or_else(|| format!("#{}", r.caller_id));
                lines.push(format!("- `{}` @ {}:{}", name, r.call_site_file, r.call_site_line));
            }
        }

        if direction == "callees" || direction == "both" {
            let callees = CallQueryEngine::query_callees(&db, target.id, depth).map_err(|e| {
                JsonRpcError {
                    code: -32603,
                    message: format!("被调用者查询失败: {}", e),
                    data: None,
                }
            })?;
            lines.push(format!("\n### 被调用者（共 {} 处）\n", callees.len()));
            for r in &callees {
                let name = db.lookup_symbol_name(r.callee_id).unwrap_or_else(|| format!("#{}", r.callee_id));
                lines.push(format!("- `{}` @ {}:{}", name, r.call_site_file, r.call_site_line));
            }
        }

        Ok(lines.join("\n"))
    }

    fn tool_analyze_file(&self, args: &serde_json::Value) -> Result<String, JsonRpcError> {
        info!("McpServer::tool_analyze_file 分析文件");
        let file_path = args.get("file_path").and_then(|v| v.as_str()).unwrap_or("");

        let db = self.open_db()?;

        let symbols = SymbolSearchEngine::search_by_file(&db, file_path).map_err(|e| {
            JsonRpcError {
                code: -32603,
                message: format!("文件符号查询失败: {}", e),
                data: None,
            }
        })?;

        let includes = IncludeQueryEngine::query_includes(&db, file_path).map_err(|e| {
            JsonRpcError {
                code: -32603,
                message: format!("包含关系查询失败: {}", e),
                data: None,
            }
        })?;

        let mut lines = vec![format!("## 文件分析: `{}`\n", file_path)];

        lines.push(format!("### 符号（共 {} 个）\n", symbols.len()));
        for s in &symbols {
            lines.push(format!(
                "- [{}] `{}` 行 {} (ID={})",
                s.kind, s.qualified_name, s.line, s.id
            ));
        }

        lines.push(format!("\n### 包含的头文件（共 {} 个）\n", includes.len()));
        for inc in &includes {
            let kind = if inc.is_system_header { "系统" } else { "项目" };
            lines.push(format!("- [{}] `{}` 行 {}", kind, inc.target_file, inc.include_line));
        }

        Ok(lines.join("\n"))
    }
}
