use cct_core::indexer::database::IndexDatabase;
use cct_core::models::relation::*;
use cct_core::models::symbol::*;
use cct_core::query::{CallQueryEngine, PathFinder, SymbolSearchEngine};
use tempfile::TempDir;

fn setup_test_db() -> (IndexDatabase, TempDir) {
    let dir = TempDir::new().unwrap();
    let db_path = dir.path().join("query_test.db");
    let mut db = IndexDatabase::open(&db_path).unwrap();
    db.initialize().unwrap();

    let symbols = vec![
        Symbol {
            id: 1,
            name: "main".into(),
            qualified_name: "main".into(),
            kind: SymbolKind::Function,
            sub_kind: None,
            file_path: "src/main.cpp".into(),
            line: 1,
            column: 1,
            end_line: Some(50),
            is_definition: true,
            return_type: Some("int".into()),
            parameters: None,
            access: None,
            attributes: None,
            project_id: "test".into(),
        },
        Symbol {
            id: 2,
            name: "init_system".into(),
            qualified_name: "core::init_system".into(),
            kind: SymbolKind::Function,
            sub_kind: None,
            file_path: "src/core.cpp".into(),
            line: 10,
            column: 1,
            end_line: Some(30),
            is_definition: true,
            return_type: Some("void".into()),
            parameters: None,
            access: Some(Access::Public),
            attributes: None,
            project_id: "test".into(),
        },
        Symbol {
            id: 3,
            name: "setup_network".into(),
            qualified_name: "net::setup_network".into(),
            kind: SymbolKind::Function,
            sub_kind: None,
            file_path: "src/network.cpp".into(),
            line: 5,
            column: 1,
            end_line: Some(25),
            is_definition: true,
            return_type: Some("bool".into()),
            parameters: None,
            access: Some(Access::Public),
            attributes: None,
            project_id: "test".into(),
        },
        Symbol {
            id: 4,
            name: "connect_socket".into(),
            qualified_name: "net::connect_socket".into(),
            kind: SymbolKind::Function,
            sub_kind: None,
            file_path: "src/network.cpp".into(),
            line: 30,
            column: 1,
            end_line: Some(60),
            is_definition: true,
            return_type: Some("int".into()),
            parameters: None,
            access: Some(Access::Private),
            attributes: None,
            project_id: "test".into(),
        },
        Symbol {
            id: 5,
            name: "MAX_CONNECTIONS".into(),
            qualified_name: "MAX_CONNECTIONS".into(),
            kind: SymbolKind::Macro,
            sub_kind: None,
            file_path: "src/config.h".into(),
            line: 1,
            column: 1,
            end_line: None,
            is_definition: true,
            return_type: None,
            parameters: None,
            access: None,
            attributes: None,
            project_id: "test".into(),
        },
        Symbol {
            id: 6,
            name: "Connection".into(),
            qualified_name: "net::Connection".into(),
            kind: SymbolKind::Type,
            sub_kind: Some("class".into()),
            file_path: "src/network.h".into(),
            line: 10,
            column: 1,
            end_line: Some(50),
            is_definition: true,
            return_type: None,
            parameters: None,
            access: Some(Access::Public),
            attributes: None,
            project_id: "test".into(),
        },
    ];
    db.insert_symbols(&symbols).unwrap();

    // main -> init_system -> setup_network -> connect_socket
    let call_relations = vec![
        CallRelation {
            id: 1,
            caller_id: 1,
            callee_id: 2,
            call_site_file: "src/main.cpp".into(),
            call_site_line: 10,
            call_site_column: 5,
            is_virtual_dispatch: false,
            is_indirect: false,
        },
        CallRelation {
            id: 2,
            caller_id: 2,
            callee_id: 3,
            call_site_file: "src/core.cpp".into(),
            call_site_line: 15,
            call_site_column: 5,
            is_virtual_dispatch: false,
            is_indirect: false,
        },
        CallRelation {
            id: 3,
            caller_id: 3,
            callee_id: 4,
            call_site_file: "src/network.cpp".into(),
            call_site_line: 10,
            call_site_column: 5,
            is_virtual_dispatch: false,
            is_indirect: false,
        },
    ];
    db.insert_call_relations(&call_relations).unwrap();

    (db, dir)
}

// ── SymbolSearchEngine ───────────────────────────────────────────────

#[test]
fn test_search_by_name_substring() {
    let (db, _dir) = setup_test_db();
    let results = SymbolSearchEngine::search(&db, "init", 10).unwrap();
    assert!(!results.is_empty());
    assert!(results.iter().any(|s| s.name == "init_system"));
}

#[test]
fn test_search_exact_match() {
    let (db, _dir) = setup_test_db();
    let results = SymbolSearchEngine::search(&db, "main", 10).unwrap();
    assert!(!results.is_empty());
    assert_eq!(results[0].name, "main", "精确匹配应排在最前");
}

#[test]
fn test_search_no_results() {
    let (db, _dir) = setup_test_db();
    let results = SymbolSearchEngine::search(&db, "nonexistent_xyz", 10).unwrap();
    assert!(results.is_empty());
}

#[test]
fn test_search_respects_limit() {
    let (db, _dir) = setup_test_db();
    let results = SymbolSearchEngine::search(&db, "net", 2).unwrap();
    assert!(results.len() <= 2);
}

#[test]
fn test_search_by_kind() {
    let (db, _dir) = setup_test_db();
    let results =
        SymbolSearchEngine::search_by_kind(&db, "Connection", SymbolKind::Type, 10).unwrap();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].kind, SymbolKind::Type);
}

#[test]
fn test_search_by_file() {
    let (db, _dir) = setup_test_db();
    let results = SymbolSearchEngine::search_by_file(&db, "src/network.cpp").unwrap();
    assert_eq!(results.len(), 2);
    assert!(results.iter().all(|s| s.file_path == "src/network.cpp"));
}

// ── CallQueryEngine ──────────────────────────────────────────────────

#[test]
fn test_query_callers_depth_1() {
    let (db, _dir) = setup_test_db();
    let callers = CallQueryEngine::query_callers(&db, 2, 1).unwrap();
    assert_eq!(callers.len(), 1);
    assert_eq!(callers[0].caller_id, 1, "init_system 的直接调用者应为 main");
}

#[test]
fn test_query_callees_depth_1() {
    let (db, _dir) = setup_test_db();
    let callees = CallQueryEngine::query_callees(&db, 1, 1).unwrap();
    assert_eq!(callees.len(), 1);
    assert_eq!(
        callees[0].callee_id, 2,
        "main 的直接被调用者应为 init_system"
    );
}

#[test]
fn test_query_callees_depth_2() {
    let (db, _dir) = setup_test_db();
    let callees = CallQueryEngine::query_callees(&db, 1, 2).unwrap();
    assert_eq!(callees.len(), 2);
    let callee_ids: Vec<i64> = callees.iter().map(|c| c.callee_id).collect();
    assert!(callee_ids.contains(&2));
    assert!(callee_ids.contains(&3));
}

#[test]
fn test_query_callers_deep() {
    let (db, _dir) = setup_test_db();
    let callers = CallQueryEngine::query_callers(&db, 4, 3).unwrap();
    assert!(callers.len() >= 3, "connect_socket 应有 3 层调用者");
}

#[test]
fn test_query_callers_no_results() {
    let (db, _dir) = setup_test_db();
    let callers = CallQueryEngine::query_callers(&db, 1, 1).unwrap();
    assert!(callers.is_empty(), "main 不应有调用者");
}

#[test]
fn test_query_callees_no_results() {
    let (db, _dir) = setup_test_db();
    let callees = CallQueryEngine::query_callees(&db, 4, 1).unwrap();
    assert!(callees.is_empty(), "connect_socket 不应有被调用者");
}

// ── PathFinder ───────────────────────────────────────────────────────

#[test]
fn test_find_shortest_path_direct() {
    let (db, _dir) = setup_test_db();
    let path = PathFinder::find_shortest_path(&db, 1, 2).unwrap();
    assert_eq!(path, vec![1, 2]);
}

#[test]
fn test_find_shortest_path_multi_hop() {
    let (db, _dir) = setup_test_db();
    let path = PathFinder::find_shortest_path(&db, 1, 4).unwrap();
    assert_eq!(path, vec![1, 2, 3, 4]);
}

#[test]
fn test_find_shortest_path_same_node() {
    let (db, _dir) = setup_test_db();
    let path = PathFinder::find_shortest_path(&db, 3, 3).unwrap();
    assert_eq!(path, vec![3]);
}

#[test]
fn test_find_shortest_path_no_path() {
    let (db, _dir) = setup_test_db();
    let path = PathFinder::find_shortest_path(&db, 4, 1).unwrap();
    assert!(path.is_empty(), "反向路径应不存在");
}

#[test]
fn test_find_shortest_path_unrelated_nodes() {
    let (db, _dir) = setup_test_db();
    let path = PathFinder::find_shortest_path(&db, 1, 5).unwrap();
    assert!(path.is_empty(), "main 到 MAX_CONNECTIONS 无调用路径");
}
