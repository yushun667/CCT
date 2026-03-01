/**
 * 编辑器 API — 文件系统操作与符号查询
 */
import { invoke } from "@tauri-apps/api/core";
import type { Symbol } from "./types";

export interface DirEntry {
  name: string;
  path: string;
  is_dir: boolean;
  size: number;
}

export async function listDirectory(dirPath: string): Promise<DirEntry[]> {
  return invoke<DirEntry[]>("list_directory", { dirPath });
}

export async function readFileContent(filePath: string): Promise<string> {
  return invoke<string>("read_file_content", { filePath });
}

export async function getFileSymbols(
  projectId: string,
  filePath: string,
): Promise<Symbol[]> {
  return invoke<Symbol[]>("get_file_symbols", { projectId, filePath });
}
