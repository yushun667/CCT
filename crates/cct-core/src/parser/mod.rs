pub mod clang_bridge;
pub mod incremental;
pub mod scheduler;

use std::path::Path;

use crate::error::CctError;
use crate::models::relation::{
    CallRelation, IncludeRelation, InheritanceRelation, ReferenceRelation,
};
use crate::models::symbol::Symbol;

/// 未解析的调用关系 — 保存原始的 caller/callee 限定名
///
/// 当被调用者定义在其他文件时，单文件解析阶段无法将名称映射为 ID，
/// 需要在全局解析完成后通过全局符号表二次解析。
#[derive(Debug, Clone)]
pub struct UnresolvedCall {
    pub caller_name: String,
    pub callee_name: String,
    pub file_path: String,
    pub line: u32,
    pub column: u32,
    pub is_virtual: bool,
    pub is_indirect: bool,
}

/// 单个文件的解析结果 — 包含提取到的所有符号和关系
#[derive(Debug, Clone, Default)]
pub struct ParseResult {
    /// 提取到的符号列表
    pub symbols: Vec<Symbol>,
    /// 函数调用关系（已在当前文件内解析的）
    pub call_relations: Vec<CallRelation>,
    /// 未解析的调用关系（被调用者不在当前文件中）
    pub unresolved_calls: Vec<UnresolvedCall>,
    /// 头文件包含关系
    pub include_relations: Vec<IncludeRelation>,
    /// 符号引用关系
    pub reference_relations: Vec<ReferenceRelation>,
    /// 类继承关系
    pub inheritance_relations: Vec<InheritanceRelation>,
}

/// 源码解析器特征 — 定义解析引擎的统一接口
///
/// 所有解析后端（Clang LibTooling、Tree-sitter 等）均需实现此特征，
/// 以便调度器通过多态方式调用不同的解析实现。
pub trait Parser: Send + Sync {
    /// 解析单个 C/C++ 源文件
    ///
    /// # 参数
    /// - `file_path`: 待解析文件的路径
    /// - `compile_args`: 编译参数列表（从编译数据库或默认配置获取）
    ///
    /// # 返回
    /// 成功时返回 `ParseResult`，包含该文件的所有符号和关系；
    /// 失败时返回 `CctError`。
    fn parse_file(
        &self,
        file_path: &Path,
        compile_args: &[String],
    ) -> Result<ParseResult, CctError>;
}
