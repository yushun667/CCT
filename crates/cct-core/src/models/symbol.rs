use serde::{Deserialize, Serialize};

pub type SymbolId = i64;

/// 符号记录 — 对应 doc/02 §3.1
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Symbol {
    pub id: SymbolId,
    pub name: String,
    pub qualified_name: String,
    pub kind: SymbolKind,
    pub sub_kind: Option<String>,
    pub file_path: String,
    pub line: u32,
    pub column: u32,
    pub end_line: Option<u32>,
    pub is_definition: bool,
    pub return_type: Option<String>,
    /// JSON 序列化的参数列表 `[(type, name)]`
    pub parameters: Option<String>,
    pub access: Option<Access>,
    /// 额外属性（JSON 对象）
    pub attributes: Option<String>,
    pub project_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum SymbolKind {
    Function,
    Variable,
    Type,
    Macro,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum Access {
    Public,
    Protected,
    Private,
}

/// 函数符号的扩展属性 — doc/02 §3.1.1
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionAttributes {
    pub is_virtual: bool,
    pub is_static: bool,
    pub is_inline: bool,
    pub storage_class: StorageClass,
    pub body_complexity: Option<u32>,
}

/// 变量符号的扩展属性 — doc/02 §3.1.2
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VariableAttributes {
    pub type_name: String,
    pub scope: VariableScope,
    pub storage_class: StorageClass,
    pub is_const: bool,
}

/// 类型符号的扩展属性 — doc/02 §3.1.3
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TypeAttributes {
    pub type_kind: TypeKind,
    pub base_classes: Vec<String>,
    pub template_params: Vec<String>,
    pub size: Option<u64>,
}

/// 宏符号的扩展属性 — doc/02 §3.1.4
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MacroAttributes {
    pub is_function_like: bool,
    pub parameters: Vec<String>,
    pub body: String,
    pub expansion: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum StorageClass {
    Extern,
    Static,
    Auto,
    Register,
    None,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum VariableScope {
    Global,
    Local,
    Member,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum TypeKind {
    Struct,
    Class,
    Enum,
    Union,
    Typedef,
}

impl std::fmt::Display for SymbolKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SymbolKind::Function => write!(f, "function"),
            SymbolKind::Variable => write!(f, "variable"),
            SymbolKind::Type => write!(f, "type"),
            SymbolKind::Macro => write!(f, "macro"),
        }
    }
}
