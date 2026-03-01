use regex::Regex;
use rusqlite::params;
use serde::{Deserialize, Serialize};
use tracing::{debug, info, warn};

use crate::error::CctError;
use crate::indexer::database::{row_to_symbol, IndexDatabase};
use crate::models::symbol::Symbol;

/// 系统调用信息 — 从 SYSCALL_DEFINE 宏中提取
///
/// # 字段
/// - `name`: 系统调用名称（如 `read`, `write`）
/// - `number`: 参数个数（SYSCALL_DEFINE 后缀数字）
/// - `params`: 参数列表（JSON 字符串）
/// - `definition_file`: 定义所在文件
/// - `definition_line`: 定义所在行号
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyscallInfo {
    pub name: String,
    pub number: u32,
    pub params: Option<String>,
    pub definition_file: String,
    pub definition_line: u32,
}

/// ioctl 命令信息 — 从 file_operations 结构体及 ioctl 处理函数中提取
///
/// # 字段
/// - `name`: 命令名称
/// - `cmd_number`: 命令编号
/// - `direction`: 数据方向（读/写/读写）
/// - `type_char`: 类型标识字符
/// - `function_handler`: 处理函数名
/// - `file`: 定义所在文件
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IoctlCommand {
    pub name: String,
    pub cmd_number: Option<u32>,
    pub direction: Option<String>,
    pub type_char: Option<String>,
    pub function_handler: String,
    pub file: String,
}

/// Linux 内核专项分析器
///
/// # 设计说明
/// 通过在索引数据库中搜索特定的符号命名模式（如 SYSCALL_DEFINE 宏、
/// file_operations 结构体），识别内核关键构件并追踪其调用路径。
pub struct LinuxKernelAnalyzer;

impl LinuxKernelAnalyzer {
    /// 搜索 SYSCALL_DEFINE[0-6] 宏定义
    ///
    /// 在 symbols 表中匹配名称以 `SYSCALL_DEFINE` 或 `__do_sys_` / `__se_sys_`
    /// 开头的宏/函数符号，提取系统调用名称与参数信息。
    ///
    /// # 参数
    /// - `db`: 索引数据库引用
    ///
    /// # 返回
    /// 匹配到的系统调用列表
    pub fn find_syscall_definitions(db: &IndexDatabase) -> Result<Vec<SyscallInfo>, CctError> {
        info!("LinuxKernelAnalyzer::find_syscall_definitions 搜索系统调用定义");

        let conn = db.conn();
        let mut stmt = conn
            .prepare(
                "SELECT name, qualified_name, kind, file_path, line, parameters
                 FROM symbols
                 WHERE (name LIKE 'SYSCALL_DEFINE%'
                    OR name LIKE '__do_sys_%'
                    OR name LIKE '__se_sys_%'
                    OR name LIKE 'sys_%')
                   AND is_definition = 1
                 ORDER BY name",
            )
            .map_err(|e| CctError::Database(e.to_string()))?;

        let re = Regex::new(r"SYSCALL_DEFINE(\d)").unwrap();

        let results: Vec<SyscallInfo> = stmt
            .query_map([], |row| {
                let name: String = row.get(0)?;
                let qualified_name: String = row.get(1)?;
                let _kind: String = row.get(2)?;
                let file_path: String = row.get(3)?;
                let line: u32 = row.get(4)?;
                let parameters: Option<String> = row.get(5)?;

                let syscall_name = extract_syscall_name(&name, &qualified_name);
                let number = re
                    .captures(&name)
                    .and_then(|c| c.get(1))
                    .and_then(|m| m.as_str().parse::<u32>().ok())
                    .unwrap_or(0);

                Ok(SyscallInfo {
                    name: syscall_name,
                    number,
                    params: parameters,
                    definition_file: file_path,
                    definition_line: line,
                })
            })
            .map_err(|e| CctError::Database(e.to_string()))?
            .filter_map(|r| r.ok())
            .collect();

        debug!(count = results.len(), "系统调用定义搜索完成");
        Ok(results)
    }

    /// 搜索 ioctl 处理函数
    ///
    /// 查找 file_operations 结构体中关联的 unlocked_ioctl / compat_ioctl
    /// 处理函数，以及名称匹配 `*_ioctl` 模式的函数。
    ///
    /// # 参数
    /// - `db`: 索引数据库引用
    ///
    /// # 返回
    /// ioctl 命令列表
    pub fn find_ioctl_handlers(db: &IndexDatabase) -> Result<Vec<IoctlCommand>, CctError> {
        info!("LinuxKernelAnalyzer::find_ioctl_handlers 搜索 ioctl 处理函数");

        let conn = db.conn();

        let mut stmt = conn
            .prepare(
                "SELECT name, qualified_name, file_path, line, parameters
                 FROM symbols
                 WHERE (name LIKE '%_ioctl'
                    OR name LIKE '%ioctl_handler%'
                    OR name LIKE 'unlocked_ioctl%'
                    OR name LIKE 'compat_ioctl%')
                   AND kind = 'function'
                   AND is_definition = 1
                 ORDER BY file_path, name",
            )
            .map_err(|e| CctError::Database(e.to_string()))?;

        let results: Vec<IoctlCommand> = stmt
            .query_map([], |row| {
                let name: String = row.get(0)?;
                let _qualified: String = row.get(1)?;
                let file_path: String = row.get(2)?;
                let _line: u32 = row.get(3)?;
                let _params: Option<String> = row.get(4)?;

                Ok(IoctlCommand {
                    name: name.clone(),
                    cmd_number: None,
                    direction: None,
                    type_char: None,
                    function_handler: name,
                    file: file_path,
                })
            })
            .map_err(|e| CctError::Database(e.to_string()))?
            .filter_map(|r| r.ok())
            .collect();

        // 搜索 file_operations 结构体以关联 ioctl 处理函数
        let mut fops_stmt = conn
            .prepare(
                "SELECT name, file_path, line
                 FROM symbols
                 WHERE kind = 'type'
                   AND (name LIKE '%file_operations%' OR name LIKE '%fops%')
                   AND is_definition = 1",
            )
            .map_err(|e| CctError::Database(e.to_string()))?;

        let fops_count: usize = fops_stmt
            .query_map([], |row| {
                let name: String = row.get(0)?;
                let file: String = row.get(1)?;
                let line: u32 = row.get(2)?;
                Ok((name, file, line))
            })
            .map_err(|e| CctError::Database(e.to_string()))?
            .filter_map(|r| r.ok())
            .count();

        debug!(
            ioctl_count = results.len(),
            fops_count = fops_count,
            "ioctl 处理函数搜索完成"
        );
        Ok(results)
    }

    /// 追踪系统调用的调用链
    ///
    /// 从指定的系统调用入口出发，沿调用关系图向下追踪至指定深度。
    ///
    /// # 参数
    /// - `db`: 索引数据库引用
    /// - `syscall_name`: 系统调用名称
    /// - `max_depth`: 最大追踪深度
    ///
    /// # 返回
    /// 调用路径上的所有符号列表（按调用顺序排列）
    pub fn trace_syscall_path(
        db: &IndexDatabase,
        syscall_name: &str,
        max_depth: u32,
    ) -> Result<Vec<Symbol>, CctError> {
        info!(
            syscall = syscall_name,
            max_depth = max_depth,
            "LinuxKernelAnalyzer::trace_syscall_path 追踪系统调用路径"
        );

        let conn = db.conn();

        // 查找系统调用入口符号
        let entry_pattern = format!("%{syscall_name}%");
        let mut stmt = conn
            .prepare(
                "SELECT id, name, qualified_name, kind, sub_kind, file_path,
                        line, column, end_line, is_definition, return_type,
                        parameters, access, attributes, project_id
                 FROM symbols
                 WHERE (name LIKE ?1 OR qualified_name LIKE ?1)
                   AND is_definition = 1
                 LIMIT 1",
            )
            .map_err(|e| CctError::Database(e.to_string()))?;

        let entry: Option<Symbol> = stmt
            .query_row(params![entry_pattern], row_to_symbol)
            .ok();

        let entry = match entry {
            Some(s) => s,
            None => {
                warn!(syscall = syscall_name, "未找到系统调用入口");
                return Ok(vec![]);
            }
        };

        let mut path = vec![entry.clone()];
        let mut visited = std::collections::HashSet::new();
        visited.insert(entry.id);

        trace_callees_recursive(conn, entry.id, max_depth, 1, &mut path, &mut visited)?;

        debug!(
            syscall = syscall_name,
            path_len = path.len(),
            "系统调用路径追踪完成"
        );
        Ok(path)
    }
}

/// 递归追踪被调用者
fn trace_callees_recursive(
    conn: &rusqlite::Connection,
    caller_id: i64,
    max_depth: u32,
    current_depth: u32,
    path: &mut Vec<Symbol>,
    visited: &mut std::collections::HashSet<i64>,
) -> Result<(), CctError> {
    if current_depth > max_depth {
        return Ok(());
    }

    let mut stmt = conn
        .prepare(
            "SELECT s.id, s.name, s.qualified_name, s.kind, s.sub_kind, s.file_path,
                    s.line, s.column, s.end_line, s.is_definition, s.return_type,
                    s.parameters, s.access, s.attributes, s.project_id
             FROM call_relations cr
             JOIN symbols s ON cr.callee_id = s.id
             WHERE cr.caller_id = ?1
             ORDER BY s.name
             LIMIT 50",
        )
        .map_err(|e| CctError::Database(e.to_string()))?;

    let callees: Vec<Symbol> = stmt
        .query_map(params![caller_id], row_to_symbol)
        .map_err(|e| CctError::Database(e.to_string()))?
        .filter_map(|r| r.ok())
        .collect();

    for callee in callees {
        if visited.contains(&callee.id) {
            continue;
        }
        visited.insert(callee.id);
        path.push(callee.clone());

        trace_callees_recursive(conn, callee.id, max_depth, current_depth + 1, path, visited)?;
    }

    Ok(())
}

/// 从符号名称中提取系统调用名
fn extract_syscall_name(name: &str, qualified_name: &str) -> String {
    let re_define = Regex::new(r"SYSCALL_DEFINE\d\((\w+)").unwrap();
    if let Some(caps) = re_define.captures(qualified_name) {
        if let Some(m) = caps.get(1) {
            return m.as_str().to_string();
        }
    }

    for prefix in &["__do_sys_", "__se_sys_", "sys_"] {
        if let Some(stripped) = name.strip_prefix(prefix) {
            return stripped.to_string();
        }
    }

    name.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_syscall_name() {
        assert_eq!(extract_syscall_name("sys_read", "sys_read"), "read");
        assert_eq!(
            extract_syscall_name("__do_sys_write", "__do_sys_write"),
            "write"
        );
        assert_eq!(
            extract_syscall_name("SYSCALL_DEFINE3", "SYSCALL_DEFINE3(read, ...)"),
            "read"
        );
    }
}
