//! 查询引擎模块 — 对应 doc/03 查询与导航需求
//!
//! # 设计说明（策略模式）
//! 每种查询类型封装为独立引擎，通过统一的 `IndexDatabase` 引用
//! 执行查询。引擎之间互不依赖，可独立扩展新的查询策略。

pub mod call_query;
pub mod include_query;
pub mod path_finder;
pub mod reference_query;
pub mod symbol_search;

pub use call_query::CallQueryEngine;
pub use include_query::IncludeQueryEngine;
pub use path_finder::PathFinder;
pub use reference_query::ReferenceQueryEngine;
pub use symbol_search::SymbolSearchEngine;
