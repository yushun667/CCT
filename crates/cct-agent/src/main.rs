//! cct-agent: 远程解析 Agent
//!
//! 通过 stdin/stdout 实现 JSON-RPC 2.0 协议的轻量级 RPC 服务器。
//! 由 CCT 桌面端通过 SSH 通道启动，负责在远程服务器上执行 C/C++ 解析。
//!
//! # 支持的方法
//! - `agent/version`: 返回 Agent 版本信息
//! - `agent/shutdown`: 安全退出进程
//! - `parse/start`: 启动解析任务
//! - `parse/cancel`: 取消解析任务
//! - `parse/status`: 查询解析状态

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::io::{self, BufRead, Write};
use tracing::{debug, error, info, warn, Level};
use tracing_subscriber::fmt;

/// JSON-RPC 2.0 请求
#[derive(Debug, Deserialize)]
struct RpcRequest {
    jsonrpc: String,
    method: String,
    #[serde(default)]
    params: Value,
    id: Value,
}

/// JSON-RPC 2.0 响应
#[derive(Debug, Serialize)]
struct RpcResponse {
    jsonrpc: String,
    result: Option<Value>,
    error: Option<RpcError>,
    id: Value,
}

/// JSON-RPC 2.0 错误对象
#[derive(Debug, Serialize)]
struct RpcError {
    code: i32,
    message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    data: Option<Value>,
}

impl RpcResponse {
    fn success(id: Value, result: Value) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            result: Some(result),
            error: None,
            id,
        }
    }

    fn error(id: Value, code: i32, message: String) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            result: None,
            error: Some(RpcError {
                code,
                message,
                data: None,
            }),
            id,
        }
    }
}

/// 分派 JSON-RPC 请求到对应的处理函数
///
/// # 参数
/// - `method`: RPC 方法名
/// - `params`: 请求参数
///
/// # 返回
/// 处理结果或错误，以及是否应当退出进程
fn dispatch(method: &str, params: &Value) -> (Result<Value, (i32, String)>, bool) {
    info!(method = %method, "dispatch — 分派 RPC 请求");

    match method {
        "agent/version" => {
            info!("处理 agent/version 请求");
            let result = serde_json::json!({
                "version": env!("CARGO_PKG_VERSION"),
                "platform": std::env::consts::OS,
                "arch": std::env::consts::ARCH,
            });
            (Ok(result), false)
        }

        "agent/shutdown" => {
            info!("处理 agent/shutdown 请求 — 即将退出");
            let result = serde_json::json!({ "ok": true });
            (Ok(result), true)
        }

        "parse/start" => {
            info!(params = %params, "处理 parse/start 请求");
            warn!("parse/start 为占位实现，不执行实际解析");
            let result = serde_json::json!({
                "status": "accepted",
                "message": "解析任务已接受（占位实现）",
            });
            (Ok(result), false)
        }

        "parse/cancel" => {
            info!("处理 parse/cancel 请求");
            warn!("parse/cancel 为占位实现");
            let result = serde_json::json!({
                "status": "cancelled",
                "message": "解析任务已取消（占位实现）",
            });
            (Ok(result), false)
        }

        "parse/status" => {
            info!("处理 parse/status 请求");
            let result = serde_json::json!({
                "status": "idle",
                "progress": null,
            });
            (Ok(result), false)
        }

        _ => {
            warn!(method = %method, "未知方法");
            (
                Err((-32601, format!("方法不存在: {}", method))),
                false,
            )
        }
    }
}

fn main() {
    // 日志输出到 stderr，stdout 保留给 JSON-RPC 通信
    fmt()
        .with_max_level(Level::DEBUG)
        .with_writer(io::stderr)
        .with_target(false)
        .init();

    info!(
        version = env!("CARGO_PKG_VERSION"),
        "cct-agent 启动 — JSON-RPC 服务器就绪"
    );

    let stdin = io::stdin();
    let mut stdout = io::stdout();

    for line in stdin.lock().lines() {
        let line = match line {
            Ok(l) => l,
            Err(e) => {
                error!(error = %e, "读取 stdin 失败");
                break;
            }
        };

        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        debug!(raw = %trimmed, "收到原始输入");

        let request: RpcRequest = match serde_json::from_str(trimmed) {
            Ok(req) => req,
            Err(e) => {
                error!(error = %e, raw = %trimmed, "JSON 解析失败");
                let resp = RpcResponse::error(Value::Null, -32700, "JSON 解析错误".to_string());
                if let Ok(json) = serde_json::to_string(&resp) {
                    let _ = writeln!(stdout, "{}", json);
                    let _ = stdout.flush();
                }
                continue;
            }
        };

        if request.jsonrpc != "2.0" {
            warn!(version = %request.jsonrpc, "无效的 JSON-RPC 版本");
        }

        info!(
            method = %request.method,
            id = %request.id,
            "处理 RPC 请求"
        );

        let (result, should_exit) = dispatch(&request.method, &request.params);

        let response = match result {
            Ok(val) => RpcResponse::success(request.id, val),
            Err((code, msg)) => RpcResponse::error(request.id, code, msg),
        };

        match serde_json::to_string(&response) {
            Ok(json) => {
                debug!(response = %json, "发送 RPC 响应");
                if let Err(e) = writeln!(stdout, "{}", json) {
                    error!(error = %e, "写入 stdout 失败");
                    break;
                }
                if let Err(e) = stdout.flush() {
                    error!(error = %e, "刷新 stdout 失败");
                    break;
                }
            }
            Err(e) => {
                error!(error = %e, "响应序列化失败");
            }
        }

        if should_exit {
            info!("收到 shutdown 请求，Agent 退出");
            break;
        }
    }

    info!("cct-agent 已停止");
}
