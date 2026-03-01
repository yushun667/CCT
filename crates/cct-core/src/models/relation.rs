use serde::{Deserialize, Serialize};

use super::symbol::SymbolId;

/// 调用关系 — 对应 doc/02 §3.2.1
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CallRelation {
    pub id: i64,
    pub caller_id: SymbolId,
    pub callee_id: SymbolId,
    pub call_site_file: String,
    pub call_site_line: u32,
    pub call_site_column: u32,
    pub is_virtual_dispatch: bool,
    pub is_indirect: bool,
}

/// 包含关系 — 对应 doc/02 §3.2.2
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IncludeRelation {
    pub id: i64,
    pub source_file: String,
    pub target_file: String,
    pub include_line: u32,
    pub is_system_header: bool,
    pub resolved_path: Option<String>,
}

/// 引用关系 — 对应 doc/02 §3.2.3
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReferenceRelation {
    pub id: i64,
    pub symbol_id: SymbolId,
    pub reference_file: String,
    pub reference_line: u32,
    pub reference_column: u32,
    pub reference_kind: RefKind,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum RefKind {
    Read,
    Write,
    Address,
    Call,
    Type,
}

/// 继承关系 — 对应 doc/02 §3.2.4
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InheritanceRelation {
    pub id: i64,
    pub derived_class_id: SymbolId,
    pub base_class_id: SymbolId,
    pub access: super::symbol::Access,
    pub is_virtual: bool,
}

/// 文件信息 — 用于增量解析跟踪
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileInfo {
    pub file_path: String,
    pub last_modified: i64,
    pub content_hash: String,
    pub parse_status: FileParseStatus,
    pub error_message: Option<String>,
    pub symbol_count: u32,
    pub parse_time_ms: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum FileParseStatus {
    Success,
    Failed,
    Skipped,
}

impl std::fmt::Display for RefKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RefKind::Read => write!(f, "read"),
            RefKind::Write => write!(f, "write"),
            RefKind::Address => write!(f, "address"),
            RefKind::Call => write!(f, "call"),
            RefKind::Type => write!(f, "type"),
        }
    }
}
