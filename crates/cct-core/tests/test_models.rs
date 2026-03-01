use cct_core::models::project::*;
use cct_core::models::symbol::*;
use cct_core::models::relation::*;
use cct_core::models::graph::*;

#[test]
fn test_project_new_local() {
    let proj = Project::new_local("my-project".into(), "/tmp/src".into());

    assert_eq!(proj.name, "my-project");
    assert_eq!(proj.source_root, "/tmp/src");
    assert_eq!(proj.project_type, ProjectType::Local);
    assert!(proj.ssh_config.is_none());
    assert!(proj.agent_config.is_none());
    assert!(proj.compile_db_path.is_none());
    assert!(proj.module_definitions.is_empty());
    assert_eq!(proj.parse_status, ParseStatus::NotStarted);
    assert!(proj.last_parse_at.is_none());
    assert!(!proj.is_remote());
}

#[test]
fn test_project_new_remote() {
    let ssh = SSHConfig::default();
    let proj = Project::new_remote("remote-proj".into(), "/opt/code".into(), ssh);

    assert_eq!(proj.name, "remote-proj");
    assert_eq!(proj.source_root, "/opt/code");
    assert_eq!(proj.project_type, ProjectType::Remote);
    assert!(proj.ssh_config.is_some());
    assert!(proj.agent_config.is_some());
    assert!(proj.is_remote());
}

#[test]
fn test_project_id_unique() {
    let a = Project::new_local("a".into(), "/a".into());
    let b = Project::new_local("b".into(), "/b".into());
    assert_ne!(a.id, b.id, "每个项目应有唯一 UUID");
}

#[test]
fn test_ssh_config_defaults() {
    let cfg = SSHConfig::default();
    assert_eq!(cfg.port, 22);
    assert!(cfg.host.is_empty());
    assert_eq!(cfg.keep_alive_interval, 30);
    assert_eq!(cfg.connect_timeout, 15);
    assert_eq!(cfg.known_hosts_policy, HostKeyPolicy::AskUser);
}

#[test]
fn test_agent_config_defaults() {
    let cfg = AgentConfig::default();
    assert_eq!(cfg.install_path, "~/.cct/agent/");
    assert_eq!(cfg.data_dir, "~/.cct/data/");
    assert_eq!(cfg.mode, AgentMode::OnDemand);
    assert!(cfg.max_threads.is_none());
    assert!(cfg.max_memory_mb.is_none());
}

#[test]
fn test_project_serialize_deserialize() {
    let proj = Project::new_local("test-proj".into(), "/tmp/test".into());
    let json = serde_json::to_string(&proj).expect("序列化失败");
    let deserialized: Project = serde_json::from_str(&json).expect("反序列化失败");

    assert_eq!(proj.id, deserialized.id);
    assert_eq!(proj.name, deserialized.name);
    assert_eq!(proj.source_root, deserialized.source_root);
    assert_eq!(proj.project_type, deserialized.project_type);
    assert_eq!(proj.parse_status, deserialized.parse_status);
}

#[test]
fn test_remote_project_serialize_deserialize() {
    let ssh = SSHConfig {
        host: "server.example.com".into(),
        port: 2222,
        username: "dev".into(),
        auth_method: SSHAuthMethod::Key {
            key_path: "/home/dev/.ssh/id_rsa".into(),
            passphrase_ref: None,
        },
        key_path: Some("/home/dev/.ssh/id_rsa".into()),
        auth_ref: "key-ref-001".into(),
        proxy_jump: Some("bastion@jump:22".into()),
        keep_alive_interval: 60,
        connect_timeout: 30,
        known_hosts_policy: HostKeyPolicy::Accept,
    };
    let proj = Project::new_remote("remote".into(), "/opt/code".into(), ssh);

    let json = serde_json::to_string_pretty(&proj).expect("序列化失败");
    let restored: Project = serde_json::from_str(&json).expect("反序列化失败");

    assert_eq!(restored.name, "remote");
    assert!(restored.is_remote());
    let ssh_cfg = restored.ssh_config.as_ref().unwrap();
    assert_eq!(ssh_cfg.host, "server.example.com");
    assert_eq!(ssh_cfg.port, 2222);
}

#[test]
fn test_symbol_serialize_deserialize() {
    let sym = Symbol {
        id: 42,
        name: "my_func".into(),
        qualified_name: "ns::my_func".into(),
        kind: SymbolKind::Function,
        sub_kind: None,
        file_path: "src/main.cpp".into(),
        line: 10,
        column: 1,
        end_line: Some(20),
        is_definition: true,
        return_type: Some("int".into()),
        parameters: Some("[(\"int\", \"x\"), (\"float\", \"y\")]".into()),
        access: Some(Access::Public),
        attributes: None,
        project_id: "proj-001".into(),
    };

    let json = serde_json::to_string(&sym).expect("序列化失败");
    let restored: Symbol = serde_json::from_str(&json).expect("反序列化失败");

    assert_eq!(restored.id, 42);
    assert_eq!(restored.name, "my_func");
    assert_eq!(restored.kind, SymbolKind::Function);
    assert_eq!(restored.access, Some(Access::Public));
    assert!(restored.is_definition);
}

#[test]
fn test_call_relation_serialize_deserialize() {
    let rel = CallRelation {
        id: 1,
        caller_id: 10,
        callee_id: 20,
        call_site_file: "src/a.cpp".into(),
        call_site_line: 15,
        call_site_column: 5,
        is_virtual_dispatch: false,
        is_indirect: false,
    };

    let json = serde_json::to_string(&rel).expect("序列化失败");
    let restored: CallRelation = serde_json::from_str(&json).expect("反序列化失败");

    assert_eq!(restored.caller_id, 10);
    assert_eq!(restored.callee_id, 20);
    assert_eq!(restored.call_site_file, "src/a.cpp");
}

#[test]
fn test_file_info_serialize_deserialize() {
    let info = FileInfo {
        file_path: "src/lib.cpp".into(),
        last_modified: 1700000000,
        content_hash: "abc123def456".into(),
        parse_status: FileParseStatus::Success,
        error_message: None,
        symbol_count: 42,
        parse_time_ms: Some(150),
    };

    let json = serde_json::to_string(&info).expect("序列化失败");
    let restored: FileInfo = serde_json::from_str(&json).expect("反序列化失败");

    assert_eq!(restored.file_path, "src/lib.cpp");
    assert_eq!(restored.parse_status, FileParseStatus::Success);
    assert_eq!(restored.symbol_count, 42);
}

#[test]
fn test_parse_statistics_default() {
    let stats = ParseStatistics::default();
    assert_eq!(stats.total_files, 0);
    assert_eq!(stats.total_symbols, 0);
    assert_eq!(stats.total_call_relations, 0);
    assert_eq!(stats.elapsed_seconds, 0.0);
}

#[test]
fn test_graph_data_serialize_deserialize() {
    let graph = GraphData {
        nodes: vec![GraphNode {
            id: "fn-1".into(),
            label: "main".into(),
            kind: GraphNodeKind::Function,
            file_path: Some("src/main.c".into()),
            line: Some(1),
            symbol_id: Some(1),
            metadata: None,
        }],
        edges: vec![GraphEdge {
            source: "fn-1".into(),
            target: "fn-2".into(),
            edge_type: GraphEdgeType::Call,
            weight: 1,
            metadata: None,
        }],
    };

    let json = serde_json::to_string(&graph).expect("序列化失败");
    let restored: GraphData = serde_json::from_str(&json).expect("反序列化失败");

    assert_eq!(restored.nodes.len(), 1);
    assert_eq!(restored.edges.len(), 1);
    assert_eq!(restored.nodes[0].kind, GraphNodeKind::Function);
    assert_eq!(restored.edges[0].edge_type, GraphEdgeType::Call);
}

#[test]
fn test_module_def_serialize() {
    let def = ModuleDef {
        name: "networking".into(),
        match_type: MatchType::Directory,
        patterns: vec!["src/net/*".into()],
        color: Some("#FF0000".into()),
        description: Some("网络通信模块".into()),
    };

    let json = serde_json::to_string(&def).expect("序列化失败");
    let restored: ModuleDef = serde_json::from_str(&json).expect("反序列化失败");

    assert_eq!(restored.name, "networking");
    assert_eq!(restored.match_type, MatchType::Directory);
    assert_eq!(restored.patterns, vec!["src/net/*"]);
}

#[test]
fn test_parse_progress_serialize() {
    let progress = ParseProgress {
        total_files: 100,
        parsed_files: 50,
        failed_files: 2,
        percentage: 50.0,
        current_file: "src/a.cpp".into(),
        symbols_found: 300,
        relations_found: 150,
        elapsed_seconds: 5.5,
        estimated_remaining: 5.5,
    };

    let json = serde_json::to_string(&progress).expect("序列化失败");
    assert!(json.contains("\"total_files\":100"));
    assert!(json.contains("\"percentage\":50.0"));
}

#[test]
fn test_inheritance_relation_serialize() {
    let rel = InheritanceRelation {
        id: 1,
        derived_class_id: 100,
        base_class_id: 200,
        access: Access::Public,
        is_virtual: true,
    };

    let json = serde_json::to_string(&rel).expect("序列化失败");
    let restored: InheritanceRelation = serde_json::from_str(&json).expect("反序列化失败");

    assert_eq!(restored.derived_class_id, 100);
    assert_eq!(restored.base_class_id, 200);
    assert!(restored.is_virtual);
}
