use cct_core::indexer::database::IndexDatabase;
use cct_core::models::relation::*;
use cct_core::models::symbol::*;
use tempfile::TempDir;

fn make_test_db() -> (IndexDatabase, TempDir) {
    let dir = TempDir::new().expect("创建临时目录失败");
    let db_path = dir.path().join("test.db");
    let db = IndexDatabase::open(&db_path).expect("打开数据库失败");
    db.initialize().expect("初始化表结构失败");
    (db, dir)
}

fn sample_symbol(id: i64, name: &str, kind: SymbolKind) -> Symbol {
    Symbol {
        id,
        name: name.into(),
        qualified_name: format!("ns::{name}"),
        kind,
        sub_kind: None,
        file_path: "src/main.cpp".into(),
        line: 10,
        column: 1,
        end_line: Some(20),
        is_definition: true,
        return_type: Some("void".into()),
        parameters: None,
        access: Some(Access::Public),
        attributes: None,
        project_id: "test-project".into(),
    }
}

#[test]
fn test_open_creates_db_file() {
    let dir = TempDir::new().unwrap();
    let db_path = dir.path().join("new.db");
    assert!(!db_path.exists());
    let _db = IndexDatabase::open(&db_path).expect("打开数据库失败");
    assert!(db_path.exists());
}

#[test]
fn test_initialize_creates_tables() {
    let (db, _dir) = make_test_db();
    let stats = db.get_statistics().expect("获取统计失败");
    assert_eq!(stats.total_files, 0);
    assert_eq!(stats.total_symbols, 0);
}

#[test]
fn test_initialize_is_idempotent() {
    let (db, _dir) = make_test_db();
    db.initialize().expect("第二次初始化应成功");
    db.initialize().expect("第三次初始化应成功");
}

#[test]
fn test_insert_and_retrieve_symbols() {
    let (mut db, _dir) = make_test_db();

    let symbols = vec![
        sample_symbol(1, "func_a", SymbolKind::Function),
        sample_symbol(2, "var_b", SymbolKind::Variable),
        sample_symbol(3, "Type_C", SymbolKind::Type),
        sample_symbol(4, "MACRO_D", SymbolKind::Macro),
    ];

    db.insert_symbols(&symbols).expect("插入符号失败");

    let stats = db.get_statistics().expect("获取统计失败");
    assert_eq!(stats.total_symbols, 4);
    assert_eq!(stats.total_functions, 1);
    assert_eq!(stats.total_variables, 1);
    assert_eq!(stats.total_types, 1);
    assert_eq!(stats.total_macros, 1);
}

#[test]
fn test_insert_call_relations_and_statistics() {
    let (mut db, _dir) = make_test_db();

    let symbols = vec![
        sample_symbol(10, "caller_fn", SymbolKind::Function),
        sample_symbol(20, "callee_fn", SymbolKind::Function),
    ];
    db.insert_symbols(&symbols).expect("插入符号失败");

    let relations = vec![CallRelation {
        id: 1,
        caller_id: 10,
        callee_id: 20,
        call_site_file: "src/main.cpp".into(),
        call_site_line: 15,
        call_site_column: 5,
        is_virtual_dispatch: false,
        is_indirect: false,
    }];
    db.insert_call_relations(&relations)
        .expect("插入调用关系失败");

    let stats = db.get_statistics().expect("获取统计失败");
    assert_eq!(stats.total_call_relations, 1);
}

#[test]
fn test_upsert_and_get_file_info() {
    let (db, _dir) = make_test_db();

    let info = FileInfo {
        file_path: "src/lib.cpp".into(),
        last_modified: 1700000000,
        content_hash: "abc123".into(),
        parse_status: FileParseStatus::Success,
        error_message: None,
        symbol_count: 10,
        parse_time_ms: Some(100),
    };
    db.upsert_file_info(&info).expect("upsert失败");

    let retrieved = db.get_file_info("src/lib.cpp").expect("查询失败");
    assert!(retrieved.is_some());
    let fi = retrieved.unwrap();
    assert_eq!(fi.file_path, "src/lib.cpp");
    assert_eq!(fi.content_hash, "abc123");
    assert_eq!(fi.parse_status, FileParseStatus::Success);
    assert_eq!(fi.symbol_count, 10);
}

#[test]
fn test_upsert_file_info_updates_existing() {
    let (db, _dir) = make_test_db();

    let info_v1 = FileInfo {
        file_path: "src/a.cpp".into(),
        last_modified: 1000,
        content_hash: "hash_v1".into(),
        parse_status: FileParseStatus::Success,
        error_message: None,
        symbol_count: 5,
        parse_time_ms: Some(50),
    };
    db.upsert_file_info(&info_v1).expect("upsert v1 失败");

    let info_v2 = FileInfo {
        file_path: "src/a.cpp".into(),
        last_modified: 2000,
        content_hash: "hash_v2".into(),
        parse_status: FileParseStatus::Failed,
        error_message: Some("解析错误".into()),
        symbol_count: 0,
        parse_time_ms: None,
    };
    db.upsert_file_info(&info_v2).expect("upsert v2 失败");

    let fi = db.get_file_info("src/a.cpp").unwrap().unwrap();
    assert_eq!(fi.content_hash, "hash_v2");
    assert_eq!(fi.parse_status, FileParseStatus::Failed);
    assert_eq!(fi.last_modified, 2000);
}

#[test]
fn test_get_file_info_not_found() {
    let (db, _dir) = make_test_db();
    let result = db.get_file_info("nonexistent.cpp").expect("查询失败");
    assert!(result.is_none());
}

#[test]
fn test_clear_file_data() {
    let (mut db, _dir) = make_test_db();

    let sym = sample_symbol(100, "fn_to_clear", SymbolKind::Function);
    db.insert_symbols(&[sym]).expect("插入符号失败");

    let info = FileInfo {
        file_path: "src/main.cpp".into(),
        last_modified: 1000,
        content_hash: "hash".into(),
        parse_status: FileParseStatus::Success,
        error_message: None,
        symbol_count: 1,
        parse_time_ms: None,
    };
    db.upsert_file_info(&info).expect("upsert失败");

    let stats_before = db.get_statistics().unwrap();
    assert_eq!(stats_before.total_symbols, 1);
    assert_eq!(stats_before.total_files, 1);

    db.clear_file_data("src/main.cpp").expect("清除失败");

    let stats_after = db.get_statistics().unwrap();
    assert_eq!(stats_after.total_symbols, 0);
    assert_eq!(stats_after.total_files, 0);
}

#[test]
fn test_clear_file_data_preserves_other_files() {
    let (mut db, _dir) = make_test_db();

    let mut sym_a = sample_symbol(1, "fn_a", SymbolKind::Function);
    sym_a.file_path = "src/a.cpp".into();
    let mut sym_b = sample_symbol(2, "fn_b", SymbolKind::Function);
    sym_b.file_path = "src/b.cpp".into();

    db.insert_symbols(&[sym_a, sym_b]).expect("插入失败");

    db.upsert_file_info(&FileInfo {
        file_path: "src/a.cpp".into(),
        last_modified: 1000,
        content_hash: "h1".into(),
        parse_status: FileParseStatus::Success,
        error_message: None,
        symbol_count: 1,
        parse_time_ms: None,
    })
    .unwrap();

    db.upsert_file_info(&FileInfo {
        file_path: "src/b.cpp".into(),
        last_modified: 1000,
        content_hash: "h2".into(),
        parse_status: FileParseStatus::Success,
        error_message: None,
        symbol_count: 1,
        parse_time_ms: None,
    })
    .unwrap();

    db.clear_file_data("src/a.cpp").unwrap();

    let stats = db.get_statistics().unwrap();
    assert_eq!(stats.total_symbols, 1, "b.cpp 的符号应保留");
    assert_eq!(stats.total_files, 1, "b.cpp 的文件信息应保留");

    assert!(db.get_file_info("src/a.cpp").unwrap().is_none());
    assert!(db.get_file_info("src/b.cpp").unwrap().is_some());
}

#[test]
fn test_get_statistics_comprehensive() {
    let (mut db, _dir) = make_test_db();

    let symbols = vec![
        sample_symbol(1, "fn1", SymbolKind::Function),
        sample_symbol(2, "fn2", SymbolKind::Function),
        sample_symbol(3, "var1", SymbolKind::Variable),
        sample_symbol(4, "MyClass", SymbolKind::Type),
        sample_symbol(5, "MY_MACRO", SymbolKind::Macro),
    ];
    db.insert_symbols(&symbols).unwrap();

    let calls = vec![
        CallRelation {
            id: 1,
            caller_id: 1,
            callee_id: 2,
            call_site_file: "src/main.cpp".into(),
            call_site_line: 10,
            call_site_column: 1,
            is_virtual_dispatch: false,
            is_indirect: false,
        },
    ];
    db.insert_call_relations(&calls).unwrap();

    db.upsert_file_info(&FileInfo {
        file_path: "src/main.cpp".into(),
        last_modified: 1000,
        content_hash: "h".into(),
        parse_status: FileParseStatus::Success,
        error_message: None,
        symbol_count: 5,
        parse_time_ms: Some(200),
    })
    .unwrap();

    let stats = db.get_statistics().unwrap();
    assert_eq!(stats.total_symbols, 5);
    assert_eq!(stats.total_functions, 2);
    assert_eq!(stats.total_variables, 1);
    assert_eq!(stats.total_types, 1);
    assert_eq!(stats.total_macros, 1);
    assert_eq!(stats.total_call_relations, 1);
    assert_eq!(stats.total_files, 1);
    assert_eq!(stats.parsed_files, 1);
    assert_eq!(stats.failed_files, 0);
}

#[test]
fn test_lookup_symbol_name() {
    let (mut db, _dir) = make_test_db();
    let sym = sample_symbol(99, "lookup_me", SymbolKind::Function);
    db.insert_symbols(&[sym]).unwrap();

    let name = db.lookup_symbol_name(99);
    assert_eq!(name, Some("ns::lookup_me".into()));

    let missing = db.lookup_symbol_name(9999);
    assert!(missing.is_none());
}

#[test]
fn test_lookup_symbol_kind() {
    let (mut db, _dir) = make_test_db();
    db.insert_symbols(&[sample_symbol(50, "my_var", SymbolKind::Variable)])
        .unwrap();

    let kind = db.lookup_symbol_kind(50);
    assert_eq!(kind, Some(SymbolKind::Variable));
}
