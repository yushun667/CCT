/**
 * 查询引擎 API — 封装 Tauri invoke 调用
 *
 * 为全局搜索、调用链查询、引用查找等功能提供类型安全的后端交互接口。
 */
import { invoke } from "@tauri-apps/api/core";
import type { Symbol, CallRelation, ReferenceRelation } from "./types";

export async function getSymbolsByIds(
  projectId: string,
  ids: number[],
): Promise<Symbol[]> {
  if (ids.length === 0) return [];
  return invoke<Symbol[]>("get_symbols_by_ids", { projectId, ids });
}

export async function searchSymbols(
  projectId: string,
  query: string,
  kind?: string,
  limit?: number,
): Promise<Symbol[]> {
  return invoke<Symbol[]>("search_symbols", {
    projectId,
    query,
    kind: kind ?? null,
    limit: limit ?? null,
  });
}

export async function queryCallers(
  projectId: string,
  symbolId: number,
  depth?: number,
): Promise<CallRelation[]> {
  return invoke<CallRelation[]>("query_callers", {
    projectId,
    symbolId,
    depth: depth ?? null,
  });
}

export async function queryCallees(
  projectId: string,
  symbolId: number,
  depth?: number,
): Promise<CallRelation[]> {
  return invoke<CallRelation[]>("query_callees", {
    projectId,
    symbolId,
    depth: depth ?? null,
  });
}

export async function queryReferences(
  projectId: string,
  symbolId: number,
): Promise<ReferenceRelation[]> {
  return invoke<ReferenceRelation[]>("query_references", {
    projectId,
    symbolId,
  });
}

export async function queryCallPath(
  projectId: string,
  fromId: number,
  toId: number,
): Promise<number[]> {
  return invoke<number[]>("query_call_path", {
    projectId,
    fromId,
    toId,
  });
}
