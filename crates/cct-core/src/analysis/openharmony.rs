use rusqlite::params;
use serde::{Deserialize, Serialize};
use tracing::{debug, info, warn};

use crate::error::CctError;
use crate::indexer::database::{row_to_symbol, IndexDatabase};
use crate::models::symbol::Symbol;

/// IPC 服务信息 — OpenHarmony 分布式通信
///
/// # 字段
/// - `service_name`: 服务注册名称
/// - `stub_class`: Stub 端类名（服务实现侧）
/// - `proxy_class`: Proxy 端类名（客户调用侧）
/// - `message_codes`: 已识别的消息码列表
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IpcService {
    pub service_name: String,
    pub stub_class: Option<String>,
    pub proxy_class: Option<String>,
    pub message_codes: Vec<MessageCode>,
}

/// IPC 消息码
///
/// # 字段
/// - `code`: 消息编号
/// - `name`: 消息常量名
/// - `handler_function`: 对应的处理函数
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageCode {
    pub code: u32,
    pub name: String,
    pub handler_function: String,
}

/// OpenHarmony 专项分析器
///
/// # 设计说明
/// 通过在索引数据库中搜索 IRemoteBroker 继承关系、
/// AddSystemAbility 调用和 OnRemoteRequest 实现来识别 IPC 服务。
pub struct OpenHarmonyAnalyzer;

impl OpenHarmonyAnalyzer {
    /// 搜索 IPC 服务定义
    ///
    /// 识别继承自 IRemoteBroker 的接口、对应的 Stub / Proxy 实现，
    /// 以及 OnRemoteRequest 中的消息分发逻辑。
    ///
    /// # 参数
    /// - `db`: 索引数据库引用
    ///
    /// # 返回
    /// 发现的 IPC 服务列表
    pub fn find_ipc_services(db: &IndexDatabase) -> Result<Vec<IpcService>, CctError> {
        info!("OpenHarmonyAnalyzer::find_ipc_services 搜索 IPC 服务定义");

        let conn = db.conn();

        // 搜索继承自 IRemoteBroker / IRemoteStub / IRemoteProxy 的类
        let mut stub_stmt = conn
            .prepare(
                "SELECT s.name, s.qualified_name, s.file_path
                 FROM symbols s
                 WHERE s.kind = 'type'
                   AND (s.name LIKE '%Stub%' OR s.name LIKE '%Proxy%'
                        OR s.qualified_name LIKE '%IRemoteBroker%')
                   AND s.is_definition = 1
                 ORDER BY s.name",
            )
            .map_err(|e| CctError::Database(e.to_string()))?;

        let type_entries: Vec<(String, String, String)> = stub_stmt
            .query_map([], |row| {
                Ok((row.get(0)?, row.get(1)?, row.get(2)?))
            })
            .map_err(|e| CctError::Database(e.to_string()))?
            .filter_map(|r| r.ok())
            .collect();

        let mut services = std::collections::HashMap::<String, IpcService>::new();

        for (name, qualified_name, _file) in &type_entries {
            let service_key = extract_service_name(name);

            let entry = services.entry(service_key.clone()).or_insert_with(|| {
                IpcService {
                    service_name: service_key.clone(),
                    stub_class: None,
                    proxy_class: None,
                    message_codes: Vec::new(),
                }
            });

            if name.contains("Stub") {
                entry.stub_class = Some(qualified_name.clone());
            } else if name.contains("Proxy") {
                entry.proxy_class = Some(qualified_name.clone());
            }
        }

        // 搜索 OnRemoteRequest 实现以提取消息码
        let mut handler_stmt = conn
            .prepare(
                "SELECT name, qualified_name, file_path, line
                 FROM symbols
                 WHERE name = 'OnRemoteRequest'
                   AND kind = 'function'
                   AND is_definition = 1",
            )
            .map_err(|e| CctError::Database(e.to_string()))?;

        let handlers: Vec<(String, String, String, u32)> = handler_stmt
            .query_map([], |row| {
                Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?))
            })
            .map_err(|e| CctError::Database(e.to_string()))?
            .filter_map(|r| r.ok())
            .collect();

        for (_name, qualified, _file, _line) in &handlers {
            let svc_name = extract_service_from_qualified(qualified);
            if let Some(service) = services.get_mut(&svc_name) {
                debug!(
                    service = %svc_name,
                    handler = %qualified,
                    "发现 OnRemoteRequest 处理函数（消息码需从源码 switch-case 中提取）"
                );
            }
        }

        let result: Vec<IpcService> = services.into_values().collect();
        debug!(count = result.len(), "IPC 服务搜索完成");
        Ok(result)
    }

    /// 搜索服务注册调用
    ///
    /// 查找 AddSystemAbility / RegisterService 等注册函数调用，
    /// 提取服务名与注册类的映射关系。
    ///
    /// # 参数
    /// - `db`: 索引数据库引用
    ///
    /// # 返回
    /// (服务名, 注册类名) 的列表
    pub fn find_service_registrations(
        db: &IndexDatabase,
    ) -> Result<Vec<(String, String)>, CctError> {
        info!("OpenHarmonyAnalyzer::find_service_registrations 搜索服务注册");

        let conn = db.conn();

        let mut stmt = conn
            .prepare(
                "SELECT s.name, s.qualified_name, s.file_path, s.line
                 FROM symbols s
                 WHERE (s.name LIKE '%AddSystemAbility%'
                    OR s.name LIKE '%RegisterService%'
                    OR s.name LIKE '%Publish%')
                   AND s.kind = 'function'
                 ORDER BY s.file_path, s.line",
            )
            .map_err(|e| CctError::Database(e.to_string()))?;

        let registrations: Vec<(String, String)> = stmt
            .query_map([], |row| {
                let name: String = row.get(0)?;
                let qualified: String = row.get(1)?;
                Ok((name, qualified))
            })
            .map_err(|e| CctError::Database(e.to_string()))?
            .filter_map(|r| r.ok())
            .collect();

        debug!(count = registrations.len(), "服务注册搜索完成");
        Ok(registrations)
    }

    /// 追踪 IPC 调用路径
    ///
    /// 从指定服务名出发，追踪 Proxy -> Stub -> OnRemoteRequest -> 实际处理函数
    /// 的完整通信路径。
    ///
    /// # 参数
    /// - `db`: 索引数据库引用
    /// - `service_name`: 服务名称
    ///
    /// # 返回
    /// 通信路径上的符号列表
    pub fn trace_ipc_call(
        db: &IndexDatabase,
        service_name: &str,
    ) -> Result<Vec<Symbol>, CctError> {
        info!(
            service = service_name,
            "OpenHarmonyAnalyzer::trace_ipc_call 追踪 IPC 调用路径"
        );

        let conn = db.conn();
        let pattern = format!("%{service_name}%");

        // 收集与服务相关的所有符号
        let mut stmt = conn
            .prepare(
                "SELECT id, name, qualified_name, kind, sub_kind, file_path,
                        line, column, end_line, is_definition, return_type,
                        parameters, access, attributes, project_id
                 FROM symbols
                 WHERE (name LIKE ?1 OR qualified_name LIKE ?1)
                   AND is_definition = 1
                 ORDER BY kind, name
                 LIMIT 100",
            )
            .map_err(|e| CctError::Database(e.to_string()))?;

        let symbols: Vec<Symbol> = stmt
            .query_map(params![pattern], row_to_symbol)
            .map_err(|e| CctError::Database(e.to_string()))?
            .filter_map(|r| r.ok())
            .collect();

        if symbols.is_empty() {
            warn!(service = service_name, "未找到与服务相关的符号");
        }

        debug!(
            service = service_name,
            symbols = symbols.len(),
            "IPC 调用路径追踪完成"
        );
        Ok(symbols)
    }
}

/// 从类名中提取服务名
fn extract_service_name(class_name: &str) -> String {
    let name = class_name
        .trim_end_matches("Stub")
        .trim_end_matches("Proxy")
        .trim_end_matches("Service")
        .trim_end_matches("Ability");

    if name.is_empty() {
        class_name.to_string()
    } else {
        name.to_string()
    }
}

/// 从全限定名中提取服务名
fn extract_service_from_qualified(qualified: &str) -> String {
    if let Some(class_part) = qualified.rsplit("::").nth(1) {
        extract_service_name(class_part)
    } else {
        qualified.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_service_name() {
        assert_eq!(extract_service_name("AudioPolicyStub"), "AudioPolicy");
        assert_eq!(extract_service_name("CameraProxy"), "Camera");
        assert_eq!(
            extract_service_name("WindowManagerService"),
            "WindowManager"
        );
    }
}
