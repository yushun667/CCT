/**
 * 图数据 API — 调用图与文件依赖图
 */
import { invoke } from "@tauri-apps/api/core";
import type { GraphData } from "./types";

export async function getCallGraph(
  projectId: string,
  rootSymbolId: number,
  depth: number,
): Promise<GraphData> {
  return invoke<GraphData>("get_call_graph", {
    projectId,
    rootSymbolId,
    depth,
  });
}

export async function getFileDependencyGraph(
  projectId: string,
): Promise<GraphData> {
  return invoke<GraphData>("get_file_dependency_graph", { projectId });
}
