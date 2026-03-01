// ClangBridgeParser — 基于 Clang LibTooling 的 C/C++ 解析后端
//
// 通过 C FFI 调用 C++ 端的 RecursiveASTVisitor，
// 遍历 AST 提取符号和关系，返回 ParseResult。
//
// ## 设计说明（策略模式）
// 作为 `Parser` 特征的具体实现，封装了 Clang LibTooling 的解析逻辑。
// 调度器持有 `dyn Parser`，通过多态在运行时选择解析后端。

use std::path::{Path, PathBuf};

use tracing::{debug, error, info, warn};

use crate::error::CctError;
use crate::models::relation::{
    CallRelation, IncludeRelation, InheritanceRelation, RefKind, ReferenceRelation,
};
use crate::models::symbol::{Access, Symbol, SymbolKind};

use super::{ParseResult, Parser};

// C FFI 声明 — 仅在 Clang 桥接编译成功时可用
#[cfg(not(no_clang_bridge))]
extern "C" {
    fn cct_parse_file(
        file_path: *const std::os::raw::c_char,
        compile_db_dir: *const std::os::raw::c_char,
        extra_args: *const std::os::raw::c_char,
        out_json: *mut *mut std::os::raw::c_char,
        out_json_len: *mut u64,
    ) -> i32;

    fn cct_free_string(ptr: *mut std::os::raw::c_char);

    fn cct_clang_version() -> *const std::os::raw::c_char;
}

/// Clang LibTooling 解析器 — 通过 C FFI 调用 C++ 端的 AST 分析
///
/// # 设计说明（策略模式）
/// 作为 `Parser` 特征的具体实现，封装了 Clang LibTooling 的解析逻辑。
/// 调度器持有 `dyn Parser`，通过多态在运行时选择解析后端，
/// 便于未来扩展其他解析器。
pub struct ClangBridgeParser {
    compile_db_path: Option<PathBuf>,
}

impl ClangBridgeParser {
    /// 创建 Clang 解析器实例
    ///
    /// # 参数
    /// - `compile_db_path`: 可选的 compile_commands.json 路径；
    ///   提供后，解析器将从中读取每个文件的编译选项。
    ///
    /// # 错误
    /// 如果提供的路径不存在，返回 `CctError::CompileDbNotFound`。
    pub fn new(compile_db_path: Option<&Path>) -> Result<Self, CctError> {
        debug!("ClangBridgeParser::new 初始化解析器");

        if let Some(path) = compile_db_path {
            if !path.exists() {
                return Err(CctError::CompileDbNotFound(
                    path.display().to_string(),
                ));
            }
            debug!(compile_db = %path.display(), "使用编译数据库");
        } else {
            debug!("未提供编译数据库，将使用默认编译参数");
        }

        #[cfg(not(no_clang_bridge))]
        {
            let ver = unsafe {
                let ptr = cct_clang_version();
                if ptr.is_null() {
                    "unknown".to_string()
                } else {
                    std::ffi::CStr::from_ptr(ptr)
                        .to_string_lossy()
                        .to_string()
                }
            };
            info!(clang_version = %ver, "Clang LibTooling 解析器就绪");
        }

        #[cfg(no_clang_bridge)]
        {
            warn!("Clang LibTooling 未编译，解析功能不可用");
        }

        Ok(Self {
            compile_db_path: compile_db_path.map(|p| p.to_path_buf()),
        })
    }

    /// 获取编译数据库路径
    pub fn compile_db_path(&self) -> Option<&Path> {
        self.compile_db_path.as_deref()
    }
}

impl Parser for ClangBridgeParser {
    fn parse_file(
        &self,
        file_path: &Path,
        _compile_args: &[String],
    ) -> Result<ParseResult, CctError> {
        debug!(
            file = %file_path.display(),
            "ClangBridgeParser::parse_file 开始解析文件"
        );

        #[cfg(no_clang_bridge)]
        {
            warn!(
                file = %file_path.display(),
                "Clang LibTooling 未编译，返回空解析结果"
            );
            return Ok(ParseResult::default());
        }

        #[cfg(not(no_clang_bridge))]
        {
            parse_file_impl(file_path, self.compile_db_path.as_deref())
        }
    }
}

/// 实际的解析实现 — 调用 C++ FFI
#[cfg(not(no_clang_bridge))]
fn parse_file_impl(
    file_path: &Path,
    compile_db_path: Option<&Path>,
) -> Result<ParseResult, CctError> {
    use std::ffi::CString;

    let file_str = file_path.display().to_string();
    let c_file = CString::new(file_str.as_str()).map_err(|_| {
        CctError::ParseFileRead("文件路径包含空字节".to_string())
    })?;

    let c_db_dir = compile_db_path
        .and_then(|p| p.parent())
        .and_then(|p| CString::new(p.display().to_string()).ok());

    let db_ptr = c_db_dir
        .as_ref()
        .map(|s| s.as_ptr())
        .unwrap_or(std::ptr::null());

    let mut out_json: *mut std::os::raw::c_char = std::ptr::null_mut();
    let mut out_json_len: u64 = 0;

    let result = unsafe {
        cct_parse_file(
            c_file.as_ptr(),
            db_ptr,
            std::ptr::null(),
            &mut out_json,
            &mut out_json_len,
        )
    };

    if out_json.is_null() {
        error!(file = %file_str, code = result, "C++ 解析返回空指针");
        return Err(CctError::ParseClangInit(
            format!("解析失败，错误码: {}", result),
        ));
    }

    let json_str = unsafe {
        let slice = std::slice::from_raw_parts(out_json as *const u8, out_json_len as usize);
        let s = String::from_utf8_lossy(slice).to_string();
        cct_free_string(out_json);
        s
    };

    if result == -2 {
        warn!(file = %file_str, "Clang 解析时触发了信号（SIGTRAP/SIGABRT），文件已跳过");
    } else if result != 0 {
        warn!(file = %file_str, code = result, "Clang 解析报告了错误（可能有诊断信息），但仍尝试提取结果");
    }

    let parsed: BridgeParseResult = serde_json::from_str(&json_str).map_err(|e| {
        error!(error = %e, "解析结果 JSON 反序列化失败");
        CctError::Internal(format!("JSON 反序列化失败: {}", e))
    })?;

    debug!(
        file = %file_str,
        symbols = parsed.symbols.len(),
        calls = parsed.calls.len(),
        includes = parsed.includes.len(),
        inherits = parsed.inherits.len(),
        references = parsed.references.len(),
        "文件解析完成"
    );

    Ok(convert_bridge_result(parsed, &file_str))
}

// ─── JSON 反序列化类型 ────────────────────────────────────────────

#[cfg(not(no_clang_bridge))]
#[derive(serde::Deserialize)]
struct BridgeParseResult {
    symbols: Vec<BridgeSymbol>,
    calls: Vec<BridgeCall>,
    includes: Vec<BridgeInclude>,
    inherits: Vec<BridgeInherit>,
    #[serde(default)]
    references: Vec<BridgeReference>,
}

#[cfg(not(no_clang_bridge))]
#[derive(serde::Deserialize)]
struct BridgeSymbol {
    name: String,
    qualified_name: String,
    kind: String,
    #[serde(default)]
    sub_kind: Option<String>,
    file_path: String,
    line: u32,
    column: u32,
    #[serde(default)]
    end_line: Option<u32>,
    is_definition: bool,
    #[serde(default)]
    return_type: Option<String>,
    #[serde(default)]
    parameters: Option<serde_json::Value>,
    #[serde(default)]
    access: Option<String>,
    #[serde(default)]
    attributes: Option<serde_json::Value>,
}

#[cfg(not(no_clang_bridge))]
#[derive(serde::Deserialize)]
struct BridgeCall {
    caller: String,
    callee: String,
    file: String,
    line: u32,
    column: u32,
    is_virtual: bool,
    is_indirect: bool,
}

#[cfg(not(no_clang_bridge))]
#[derive(serde::Deserialize)]
struct BridgeInclude {
    source_file: String,
    target_file: String,
    line: u32,
    is_system: bool,
    #[serde(default)]
    resolved_path: Option<String>,
}

#[cfg(not(no_clang_bridge))]
#[derive(serde::Deserialize)]
struct BridgeInherit {
    derived: String,
    base: String,
    access: String,
    is_virtual: bool,
}

#[cfg(not(no_clang_bridge))]
#[derive(serde::Deserialize)]
struct BridgeReference {
    symbol_name: String,
    file: String,
    line: u32,
    column: u32,
    ref_kind: String,
}

// ─── 结果转换 ────────────────────────────────────────────────────

#[cfg(not(no_clang_bridge))]
fn convert_bridge_result(parsed: BridgeParseResult, _file_path: &str) -> ParseResult {
    use std::sync::atomic::{AtomicI64, Ordering};
    static ID_COUNTER: AtomicI64 = AtomicI64::new(1);

    let mut symbols = Vec::with_capacity(parsed.symbols.len());
    let mut symbol_name_to_id = std::collections::HashMap::new();

    for bs in &parsed.symbols {
        let id = ID_COUNTER.fetch_add(1, Ordering::Relaxed);
        symbol_name_to_id.insert(bs.qualified_name.clone(), id);

        let kind = match bs.kind.as_str() {
            "function" => SymbolKind::Function,
            "variable" => SymbolKind::Variable,
            "type" => SymbolKind::Type,
            "macro" => SymbolKind::Macro,
            _ => SymbolKind::Function,
        };

        let access = bs.access.as_deref().map(|a| match a {
            "public" => Access::Public,
            "protected" => Access::Protected,
            "private" => Access::Private,
            _ => Access::Public,
        });

        symbols.push(Symbol {
            id,
            name: bs.name.clone(),
            qualified_name: bs.qualified_name.clone(),
            kind,
            sub_kind: bs.sub_kind.clone(),
            file_path: bs.file_path.clone(),
            line: bs.line,
            column: bs.column,
            end_line: bs.end_line,
            is_definition: bs.is_definition,
            return_type: bs.return_type.clone(),
            parameters: bs.parameters.as_ref().map(|p| p.to_string()),
            access,
            attributes: bs.attributes.as_ref().map(|a| a.to_string()),
            project_id: String::new(), // filled by caller
        });
    }

    let mut call_relations = Vec::new();
    let mut unresolved_calls = Vec::new();

    for bc in &parsed.calls {
        let caller_id = symbol_name_to_id.get(&bc.caller).copied();
        let callee_id = symbol_name_to_id.get(&bc.callee).copied();

        match (caller_id, callee_id) {
            (Some(crid), Some(ceid)) => {
                call_relations.push(CallRelation {
                    id: ID_COUNTER.fetch_add(1, Ordering::Relaxed),
                    caller_id: crid,
                    callee_id: ceid,
                    call_site_file: bc.file.clone(),
                    call_site_line: bc.line,
                    call_site_column: bc.column,
                    is_virtual_dispatch: bc.is_virtual,
                    is_indirect: bc.is_indirect,
                });
            }
            _ => {
                unresolved_calls.push(super::UnresolvedCall {
                    caller_name: bc.caller.clone(),
                    callee_name: bc.callee.clone(),
                    file_path: bc.file.clone(),
                    line: bc.line,
                    column: bc.column,
                    is_virtual: bc.is_virtual,
                    is_indirect: bc.is_indirect,
                });
            }
        }
    }

    let include_relations: Vec<IncludeRelation> = parsed
        .includes
        .iter()
        .map(|bi| IncludeRelation {
            id: ID_COUNTER.fetch_add(1, Ordering::Relaxed),
            source_file: bi.source_file.clone(),
            target_file: bi.target_file.clone(),
            include_line: bi.line,
            is_system_header: bi.is_system,
            resolved_path: bi.resolved_path.clone(),
        })
        .collect();

    let inheritance_relations: Vec<InheritanceRelation> = parsed
        .inherits
        .iter()
        .filter_map(|bi| {
            let derived_id = *symbol_name_to_id.get(&bi.derived)?;
            let base_id = *symbol_name_to_id.get(&bi.base)?;
            let access = match bi.access.as_str() {
                "public" => Access::Public,
                "protected" => Access::Protected,
                "private" => Access::Private,
                _ => Access::Public,
            };
            Some(InheritanceRelation {
                id: ID_COUNTER.fetch_add(1, Ordering::Relaxed),
                derived_class_id: derived_id,
                base_class_id: base_id,
                access,
                is_virtual: bi.is_virtual,
            })
        })
        .collect();

    let reference_relations: Vec<ReferenceRelation> = parsed
        .references
        .iter()
        .filter_map(|br| {
            let symbol_id = *symbol_name_to_id.get(&br.symbol_name)?;
            let kind = match br.ref_kind.as_str() {
                "read" => RefKind::Read,
                "write" => RefKind::Write,
                "address" => RefKind::Address,
                "call" => RefKind::Call,
                "type" => RefKind::Type,
                _ => RefKind::Read,
            };
            Some(ReferenceRelation {
                id: ID_COUNTER.fetch_add(1, Ordering::Relaxed),
                symbol_id,
                reference_file: br.file.clone(),
                reference_line: br.line,
                reference_column: br.column,
                reference_kind: kind,
            })
        })
        .collect();

    ParseResult {
        symbols,
        call_relations,
        unresolved_calls,
        include_relations,
        reference_relations,
        inheritance_relations,
    }
}
