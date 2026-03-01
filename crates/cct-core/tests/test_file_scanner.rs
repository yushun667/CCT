use std::fs;

use cct_core::indexer::file_scanner;
use tempfile::TempDir;

fn create_test_tree() -> TempDir {
    let dir = TempDir::new().unwrap();
    let root = dir.path();

    fs::create_dir_all(root.join("src")).unwrap();
    fs::create_dir_all(root.join("include")).unwrap();
    fs::create_dir_all(root.join("build")).unwrap();

    fs::write(root.join("src/main.cpp"), "int main() { return 0; }").unwrap();
    fs::write(root.join("src/util.c"), "void util() {}").unwrap();
    fs::write(root.join("src/helper.cc"), "void helper() {}").unwrap();
    fs::write(root.join("src/extra.cxx"), "void extra() {}").unwrap();
    fs::write(root.join("include/util.h"), "#pragma once").unwrap();
    fs::write(root.join("include/api.hpp"), "#pragma once").unwrap();
    fs::write(root.join("include/compat.hh"), "#pragma once").unwrap();
    fs::write(root.join("include/legacy.hxx"), "#pragma once").unwrap();

    fs::write(root.join("build/output.o"), "binary-data").unwrap();
    fs::write(root.join("src/README.md"), "docs").unwrap();
    fs::write(root.join("src/Makefile"), "all:").unwrap();

    dir
}

#[test]
fn test_scan_finds_all_c_cpp_files() {
    let dir = create_test_tree();
    let extensions = &["c", "cc", "cpp", "cxx", "h", "hh", "hpp", "hxx"];
    let files = file_scanner::scan_source_files(dir.path(), extensions);

    assert_eq!(files.len(), 8, "应找到 8 个 C/C++ 文件");
}

#[test]
fn test_scan_excludes_non_source_files() {
    let dir = create_test_tree();
    let extensions = &["c", "cc", "cpp", "cxx", "h", "hh", "hpp", "hxx"];
    let files = file_scanner::scan_source_files(dir.path(), extensions);

    for f in &files {
        let ext = f.extension().unwrap().to_str().unwrap().to_lowercase();
        assert!(
            extensions.contains(&ext.as_str()),
            "不应包含 {} 扩展名",
            ext
        );
    }
}

#[test]
fn test_scan_returns_sorted_paths() {
    let dir = create_test_tree();
    let extensions = &["c", "cpp"];
    let files = file_scanner::scan_source_files(dir.path(), extensions);

    let paths: Vec<String> = files.iter().map(|f| f.display().to_string()).collect();
    let mut sorted = paths.clone();
    sorted.sort();
    assert_eq!(paths, sorted, "结果应按路径排序");
}

#[test]
fn test_scan_empty_directory() {
    let dir = TempDir::new().unwrap();
    let files = file_scanner::scan_source_files(dir.path(), &["c", "cpp"]);
    assert!(files.is_empty());
}

#[test]
fn test_scan_only_specific_extensions() {
    let dir = create_test_tree();
    let files = file_scanner::scan_source_files(dir.path(), &["h", "hpp"]);
    assert_eq!(files.len(), 2);
}

#[test]
fn test_compute_file_hash_consistency() {
    let dir = TempDir::new().unwrap();
    let file_path = dir.path().join("test.c");
    fs::write(&file_path, "int main() { return 0; }").unwrap();

    let hash1 = file_scanner::compute_file_hash(&file_path).unwrap();
    let hash2 = file_scanner::compute_file_hash(&file_path).unwrap();

    assert_eq!(hash1, hash2, "相同内容的哈希应一致");
    assert!(!hash1.is_empty());
    assert_eq!(hash1.len(), 64, "SHA-256 十六进制应为 64 字符");
}

#[test]
fn test_compute_file_hash_different_content() {
    let dir = TempDir::new().unwrap();
    let file_a = dir.path().join("a.c");
    let file_b = dir.path().join("b.c");
    fs::write(&file_a, "int a = 1;").unwrap();
    fs::write(&file_b, "int b = 2;").unwrap();

    let hash_a = file_scanner::compute_file_hash(&file_a).unwrap();
    let hash_b = file_scanner::compute_file_hash(&file_b).unwrap();

    assert_ne!(hash_a, hash_b, "不同内容的哈希应不同");
}

#[test]
fn test_compute_file_hash_nonexistent_file() {
    let dir = TempDir::new().unwrap();
    let result = file_scanner::compute_file_hash(&dir.path().join("no_such_file.c"));
    assert!(result.is_err());
}

#[test]
fn test_scan_nested_directories() {
    let dir = TempDir::new().unwrap();
    let root = dir.path();

    fs::create_dir_all(root.join("a/b/c")).unwrap();
    fs::write(root.join("a/top.c"), "void top(){}").unwrap();
    fs::write(root.join("a/b/mid.c"), "void mid(){}").unwrap();
    fs::write(root.join("a/b/c/deep.c"), "void deep(){}").unwrap();

    let files = file_scanner::scan_source_files(root, &["c"]);
    assert_eq!(files.len(), 3, "应递归扫描到所有层级的 .c 文件");
}
