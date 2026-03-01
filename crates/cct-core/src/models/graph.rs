use serde::{Deserialize, Serialize};

use super::symbol::SymbolId;

/// 图节点 — 用于可视化和查询结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphNode {
    pub id: String,
    pub label: String,
    pub kind: GraphNodeKind,
    pub file_path: Option<String>,
    pub line: Option<u32>,
    pub symbol_id: Option<SymbolId>,
    pub metadata: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum GraphNodeKind {
    Function,
    File,
    Module,
    Type,
}

/// 图边
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphEdge {
    pub source: String,
    pub target: String,
    pub edge_type: GraphEdgeType,
    pub weight: u32,
    pub metadata: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum GraphEdgeType {
    Call,
    Include,
    Reference,
    Inheritance,
    Dependency,
}

/// 完整的图数据结构
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphData {
    pub nodes: Vec<GraphNode>,
    pub edges: Vec<GraphEdge>,
}

/// 解析统计信息
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ParseStatistics {
    pub total_files: u64,
    pub parsed_files: u64,
    pub failed_files: u64,
    pub total_symbols: u64,
    pub total_functions: u64,
    pub total_variables: u64,
    pub total_types: u64,
    pub total_macros: u64,
    pub total_call_relations: u64,
    pub total_include_relations: u64,
    pub total_reference_relations: u64,
    pub total_inheritance_relations: u64,
    pub elapsed_seconds: f64,
}
