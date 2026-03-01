/**
 * 项目管理与解析控制 API — 封装 Tauri invoke 调用
 *
 * 为 Pinia store 和组件提供类型安全的后端交互接口，
 * 将 Tauri IPC 细节隔离在此层。
 */
import { invoke } from "@tauri-apps/api/core";
import type { Project, ParseStatistics } from "./types";

// ── 项目 CRUD ──────────────────────────────────────────────────────

export async function createLocalProject(
  name: string,
  sourceRoot: string,
): Promise<Project> {
  return invoke<Project>("create_local_project", {
    name,
    sourceRoot,
  });
}

export async function listProjects(): Promise<Project[]> {
  return invoke<Project[]>("list_projects");
}

export async function getProject(projectId: string): Promise<Project> {
  return invoke<Project>("get_project", { projectId });
}

export async function updateProject(
  projectId: string,
  name?: string,
  compileDbPath?: string,
): Promise<Project> {
  return invoke<Project>("update_project", {
    projectId,
    name: name ?? null,
    compileDbPath: compileDbPath ?? null,
  });
}

export async function deleteProject(projectId: string): Promise<void> {
  return invoke<void>("delete_project", { projectId });
}

// ── 解析控制 ──────────────────────────────────────────────────────

export async function startFullParse(projectId: string): Promise<void> {
  return invoke<void>("start_full_parse", { projectId });
}

export async function cancelParse(projectId: string): Promise<void> {
  return invoke<void>("cancel_parse", { projectId });
}

export async function getParseStatus(projectId: string): Promise<string> {
  return invoke<string>("get_parse_status", { projectId });
}

export async function getParseStatistics(
  projectId: string,
): Promise<ParseStatistics> {
  return invoke<ParseStatistics>("get_parse_statistics", { projectId });
}

// ── 远程项目 ──────────────────────────────────────────────────────

export async function testSshConnection(
  host: string,
  port: number,
  username: string,
): Promise<boolean> {
  return invoke<boolean>("test_ssh_connection", { host, port, username });
}

export interface RemoteFileEntry {
  name: string;
  path: string;
  is_dir: boolean;
  size: number;
  modified: number;
}

export async function browseRemoteDir(
  projectId: string,
  path: string,
): Promise<RemoteFileEntry[]> {
  return invoke<RemoteFileEntry[]>("browse_remote_dir", { projectId, path });
}

export async function deployAgent(projectId: string): Promise<void> {
  return invoke<void>("deploy_agent", { projectId });
}

export interface RemoteStatus {
  ssh_state: "Disconnected" | "Connecting" | "Connected" | "Error";
  agent_state:
    | "NotInstalled"
    | "Stopped"
    | "Starting"
    | "Running"
    | "Error";
  agent_version: string | null;
  server_info: {
    hostname: string;
    os: string;
    cpu_cores: number;
    total_memory_mb: number;
    available_memory_mb: number;
    disk_free_mb: number;
  } | null;
  last_error: string | null;
}

export async function getRemoteStatus(
  projectId: string,
): Promise<RemoteStatus> {
  return invoke<RemoteStatus>("get_remote_status", { projectId });
}
