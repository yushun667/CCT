/**
 * 编辑器 API — 文件内容读取与符号查询
 */
import { invoke } from "@tauri-apps/api/core";
import type { Symbol } from "./types";

export async function readFileContent(filePath: string): Promise<string> {
  return invoke<string>("read_file_content", { filePath });
}

export async function getFileSymbols(
  projectId: string,
  filePath: string,
): Promise<Symbol[]> {
  return invoke<Symbol[]>("get_file_symbols", { projectId, filePath });
}
