/// @file bridge.h
/// @brief C FFI 桥接头文件 — Clang LibTooling 解析引擎对外接口
///
/// 通过 extern "C" 函数将 C++ LibTooling 的 AST 遍历结果
/// 以 JSON 字符串的形式传递给 Rust 端，避免复杂的跨语言类型映射。

#ifndef CCT_BRIDGE_H
#define CCT_BRIDGE_H

#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

/// 解析单个 C/C++ 源文件，返回 JSON 格式的解析结果
///
/// @param file_path       待解析文件的绝对路径 (UTF-8)
/// @param compile_db_dir  compile_commands.json 所在目录（可为 NULL）
/// @param extra_args      额外编译参数，以 \0 分隔，最后以 \0\0 结束（可为 NULL）
/// @param out_json        [out] 输出 JSON 字符串指针，调用方需用 cct_free_string 释放
/// @param out_json_len    [out] 输出 JSON 字符串长度
/// @return 0 成功，非 0 错误码
int32_t cct_parse_file(
    const char *file_path,
    const char *compile_db_dir,
    const char *extra_args,
    char **out_json,
    uint64_t *out_json_len
);

/// 释放 cct_parse_file 分配的 JSON 字符串
void cct_free_string(char *ptr);

/// 获取 Clang 版本信息
/// @return 静态字符串，无需释放
const char *cct_clang_version(void);

#ifdef __cplusplus
}
#endif

#endif // CCT_BRIDGE_H
