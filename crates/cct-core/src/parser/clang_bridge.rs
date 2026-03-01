// ClangBridgeParser — 基于 Clang LibTooling 的 C/C++ 解析后端
//
// 实际的 LibTooling 集成需要通过 cxx 桥接 C++ 代码，并依赖 LLVM/Clang 开发库。
// 当前版本为占位实现，在 LLVM dev libs 可用后将替换为完整的 AST 解析逻辑。
// 集成方案详见 doc/02 §附录 B。

use std::path::{Path, PathBuf};

use tracing::{debug, warn};

use crate::error::CctError;

use super::{ParseResult, Parser};

/// Clang LibTooling 解析器 — 通过 cxx 桥接调用 C++ 端的 Clang AST 分析
///
/// # 设计说明（策略模式）
/// 作为 `Parser` 特征的具体实现，封装了 Clang LibTooling 的解析逻辑。
/// 调度器持有 `dyn Parser`，通过多态在运行时选择解析后端，
/// 便于未来切换到 Tree-sitter 等替代方案。
pub struct ClangBridgeParser {
    /// 编译数据库路径（compile_commands.json）
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
        compile_args: &[String],
    ) -> Result<ParseResult, CctError> {
        debug!(
            file = %file_path.display(),
            args_count = compile_args.len(),
            "ClangBridgeParser::parse_file 开始解析文件"
        );

        // TODO: 当 LLVM dev libs 可用后，此处将通过 cxx 调用 C++ 端的
        // RecursiveASTVisitor 遍历 AST，提取符号和关系。
        // 集成步骤：
        //   1. 在 crates/cct-core/cpp/ 下编写 C++ ASTVisitor
        //   2. 使用 cxx::bridge 定义 Rust ↔ C++ 接口
        //   3. 在 build.rs 中配置 cxx_build 并链接 libclang
        warn!(
            file = %file_path.display(),
            "Clang LibTooling 尚未集成，返回空解析结果"
        );

        Ok(ParseResult::default())
    }
}
