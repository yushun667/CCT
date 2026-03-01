pub mod clang_bridge;
pub mod incremental;
pub mod scheduler;

use std::path::Path;

use crate::error::CctError;
use crate::models::relation::{
    CallRelation, IncludeRelation, InheritanceRelation, ReferenceRelation,
};
use crate::models::symbol::Symbol;

/// 单个文件的解析结果 — 包含提取到的所有符号和关系
#[derive(Debug, Clone, Default)]
pub struct ParseResult {
    /// 提取到的符号列表
    pub symbols: Vec<Symbol>,
    /// 函数调用关系
    pub call_relations: Vec<CallRelation>,
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
