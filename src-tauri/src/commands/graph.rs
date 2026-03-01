use std::collections::HashSet;
use std::path::PathBuf;

use tracing::{debug, info};

use cct_core::error::CctError;
use cct_core::indexer::database::IndexDatabase;
use cct_core::models::graph::{GraphData, GraphEdge, GraphEdgeType, GraphNode, GraphNodeKind};
use cct_core::models::symbol::SymbolKind;
use cct_core::query::{CallQueryEngine, IncludeQueryEngine};

fn open_project_db(project_id: &str) -> Result<IndexDatabase, CctError> {
    let db_path = dirs::data_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("cct")
        .join("index")
        .join(format!("{project_id}.db"));

    if !db_path.exists() {
        return Err(CctError::Database(format!(
            "索引数据库不存在: {}",
            db_path.display()
        )));
    }
    IndexDatabase::open(&db_path)
}

/// 获取以指定函数为根的调用图
///
/// # 参数
/// - `root_symbol_id`: 根函数符号 ID
/// - `depth`: 展开深度
///
/// # 返回
/// 包含节点和边的 GraphData，可直接用于前端可视化
#[tauri::command]
pub fn get_call_graph(
    project_id: String,
    root_symbol_id: i64,
    depth: u32,
) -> Result<GraphData, CctError> {
    info!(
        project_id = %project_id,
        root = root_symbol_id,
        depth = depth,
        "Tauri Command: get_call_graph"
    );

    let db = open_project_db(&project_id)?;
    let relations = CallQueryEngine::query_callees(&db, root_symbol_id, depth)?;

    let mut symbol_ids = HashSet::new();
    symbol_ids.insert(root_symbol_id);
    for r in &relations {
        symbol_ids.insert(r.caller_id);
        symbol_ids.insert(r.callee_id);
    }

    let mut nodes = Vec::new();

    for &sid in &symbol_ids {
        let label = db
            .lookup_symbol_name(sid)
            .unwrap_or_else(|| format!("symbol_{sid}"));

        let kind = db.lookup_symbol_kind(sid);
        nodes.push(GraphNode {
            id: sid.to_string(),
            label,
            kind: match kind {
                Some(SymbolKind::Function) => GraphNodeKind::Function,
                Some(SymbolKind::Type) => GraphNodeKind::Type,
                _ => GraphNodeKind::Function,
            },
            file_path: db.lookup_symbol_file(sid),
            line: db.lookup_symbol_line(sid),
            symbol_id: Some(sid),
            metadata: None,
        });
    }

    let edges: Vec<GraphEdge> = relations
        .iter()
        .map(|r| GraphEdge {
            source: r.caller_id.to_string(),
            target: r.callee_id.to_string(),
            edge_type: GraphEdgeType::Call,
            weight: 1,
            metadata: None,
        })
        .collect();

    debug!(
        nodes = nodes.len(),
        edges = edges.len(),
        "调用图构建完成"
    );

    Ok(GraphData { nodes, edges })
}

/// 获取文件依赖图（基于 #include 关系）
///
/// # 返回
/// 节点为文件，边为 include 关系的图数据
#[tauri::command]
pub fn get_file_dependency_graph(project_id: String) -> Result<GraphData, CctError> {
    info!(
        project_id = %project_id,
        "Tauri Command: get_file_dependency_graph"
    );

    let db = open_project_db(&project_id)?;
    let includes = IncludeQueryEngine::query_all(&db)?;

    let mut file_set = HashSet::new();
    for inc in &includes {
        file_set.insert(inc.source_file.clone());
        file_set.insert(inc.target_file.clone());
    }

    let nodes: Vec<GraphNode> = file_set
        .iter()
        .map(|f| {
            let label = f
                .rsplit('/')
                .next()
                .or_else(|| f.rsplit('\\').next())
                .unwrap_or(f)
                .to_string();
            GraphNode {
                id: f.clone(),
                label,
                kind: GraphNodeKind::File,
                file_path: Some(f.clone()),
                line: None,
                symbol_id: None,
                metadata: None,
            }
        })
        .collect();

    let edges: Vec<GraphEdge> = includes
        .iter()
        .map(|inc| GraphEdge {
            source: inc.source_file.clone(),
            target: inc.target_file.clone(),
            edge_type: GraphEdgeType::Include,
            weight: 1,
            metadata: None,
        })
        .collect();

    debug!(
        nodes = nodes.len(),
        edges = edges.len(),
        "文件依赖图构建完成"
    );

    Ok(GraphData { nodes, edges })
}
