/** 前后端共享类型定义 — 对应 Rust cct-core 的数据模型 */

export interface Project {
  id: string;
  name: string;
  project_type: "Local" | "Remote";
  source_root: string;
  ssh_config: SSHConfig | null;
  agent_config: AgentConfig | null;
  compile_db_path: string | null;
  module_definitions: ModuleDef[];
  created_at: string;
  updated_at: string;
  last_parse_at: string | null;
  parse_status: ParseStatus;
}

export type ParseStatus = "NotStarted" | "InProgress" | "Completed" | "Failed";

export interface SSHConfig {
  host: string;
  port: number;
  username: string;
  auth_method: SSHAuthMethod;
  key_path: string | null;
  auth_ref: string;
  proxy_jump: string | null;
  keep_alive_interval: number;
  connect_timeout: number;
  known_hosts_policy: "Accept" | "Reject" | "AskUser";
}

export type SSHAuthMethod =
  | { Key: { key_path: string; passphrase_ref: string | null } }
  | { Password: { password_ref: string } }
  | "Agent";

export interface AgentConfig {
  install_path: string;
  data_dir: string;
  mode: "OnDemand" | "Daemon";
  max_threads: number | null;
  max_memory_mb: number | null;
  version: string | null;
}

export interface ModuleDef {
  name: string;
  match_type: "Directory" | "FileList" | "Regex";
  patterns: string[];
  color: string | null;
  description: string | null;
}

export interface ParseProgress {
  total_files: number;
  parsed_files: number;
  failed_files: number;
  percentage: number;
  current_file: string;
  symbols_found: number;
  relations_found: number;
  elapsed_seconds: number;
  estimated_remaining: number;
}

export interface ParseStatistics {
  total_files: number;
  parsed_files: number;
  failed_files: number;
  total_symbols: number;
  total_functions: number;
  total_variables: number;
  total_types: number;
  total_macros: number;
  total_call_relations: number;
  total_include_relations: number;
  total_reference_relations: number;
  total_inheritance_relations: number;
  elapsed_seconds: number;
}

// ── M3: 查询与编辑器类型 ──────────────────────────────────────────

export type SymbolKind = "Function" | "Variable" | "Type" | "Macro";

export interface Symbol {
  id: number;
  name: string;
  qualified_name: string;
  kind: SymbolKind;
  sub_kind: string | null;
  file_path: string;
  line: number;
  column: number;
  end_line: number | null;
  is_definition: boolean;
  return_type: string | null;
  parameters: string | null;
  access: "Public" | "Protected" | "Private" | null;
  attributes: string | null;
  project_id: string;
}

export interface CallRelation {
  id: number;
  caller_id: number;
  callee_id: number;
  call_site_file: string;
  call_site_line: number;
  call_site_column: number;
  is_virtual_dispatch: boolean;
  is_indirect: boolean;
}

export type RefKind = "Read" | "Write" | "Address" | "Call" | "Type";

export interface ReferenceRelation {
  id: number;
  symbol_id: number;
  reference_file: string;
  reference_line: number;
  reference_column: number;
  reference_kind: RefKind;
}

export interface IncludeRelation {
  id: number;
  source_file: string;
  target_file: string;
  include_line: number;
  is_system_header: boolean;
  resolved_path: string | null;
}

// ── M4: 图数据类型 ────────────────────────────────────────────────

export type GraphNodeKind = "Function" | "File" | "Module" | "Type";
export type GraphEdgeType =
  | "Call"
  | "Include"
  | "Reference"
  | "Inheritance"
  | "Dependency";

export interface GraphNode {
  id: string;
  label: string;
  kind: GraphNodeKind;
  file_path: string | null;
  line: number | null;
  symbol_id: number | null;
  metadata: Record<string, unknown> | null;
}

export interface GraphEdge {
  source: string;
  target: string;
  edge_type: GraphEdgeType;
  weight: number;
  metadata: Record<string, unknown> | null;
}

export interface GraphData {
  nodes: GraphNode[];
  edges: GraphEdge[];
}

// ── 编辑器 Tab 类型 ──────────────────────────────────────────────

export interface EditorFile {
  filePath: string;
  fileName: string;
  content: string;
  language: string;
}

export interface AppConfig {
  data_dir: string;
  log: {
    level: string;
    max_file_size_mb: number;
    max_file_count: number;
    retention_days: number;
  };
  parse: {
    max_threads: number | null;
    max_memory_mb: number | null;
    file_extensions: string[];
  };
  ui: {
    theme: "Light" | "Dark" | "System";
    language: string;
    font_size: number;
    sidebar_width: number;
  };
  ai: {
    provider: string | null;
    model: string | null;
    api_key_ref: string | null;
    base_url: string | null;
    privacy_mode: "Full" | "Anonymized" | "Local";
  };
}
