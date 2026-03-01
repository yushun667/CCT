use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};
use tracing::{debug, error, info, trace};
use walkdir::WalkDir;

use crate::error::CctError;

/// 扫描源码目录，收集匹配指定扩展名的文件列表
///
/// # 参数
/// - `root`: 源码根目录
/// - `extensions`: 允许的文件扩展名列表（不含 `.`），如 `["c", "cpp", "h"]`
/// - `extra_excluded`: 用户自定义的额外排除目录名
///
/// # 返回
/// 按路径排序的文件列表
pub fn scan_source_files(
    root: &Path,
    extensions: &[&str],
    extra_excluded: &[String],
) -> Vec<PathBuf> {
    info!(
        root = %root.display(),
        extensions = ?extensions,
        extra_excluded = ?extra_excluded,
        "scan_source_files 开始扫描源码目录"
    );

    let mut files = Vec::new();

    let builtin_skip: &[&str] = &[
        ".git", ".svn", ".hg", "node_modules", "__pycache__",
        ".build", ".cache", ".cct",
        "test", "tests", "unittests", "unittest", "testing",
        "benchmarks", "benchmark", "examples", "example",
    ];

    for entry in WalkDir::new(root)
        .follow_links(true)
        .into_iter()
        .filter_entry(|e| {
            if e.file_type().is_dir() {
                let name = e.file_name().to_string_lossy();
                if builtin_skip.iter().any(|&s| s == &*name) {
                    return false;
                }
                if extra_excluded.iter().any(|s| s == &*name) {
                    return false;
                }
            }
            true
        })
        .filter_map(|e| e.ok())
    {
        if !entry.file_type().is_file() {
            continue;
        }

        let path = entry.path();

        if let Some(fname) = path.file_name().and_then(|n| n.to_str()) {
            if fname.starts_with("._") {
                continue;
            }
        }

        if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
            let ext_lower = ext.to_lowercase();
            if extensions.iter().any(|&allowed| allowed == ext_lower) {
                trace!(file = %path.display(), "发现匹配的源文件");
                files.push(path.to_path_buf());
            }
        }
    }

    files.sort();

    debug!(count = files.len(), "扫描完成");
    files
}

/// 计算文件内容的 SHA-256 哈希值
///
/// # 参数
/// - `path`: 文件路径
///
/// # 返回
/// 十六进制编码的 SHA-256 哈希字符串
pub fn compute_file_hash(path: &Path) -> Result<String, CctError> {
    debug!(file = %path.display(), "compute_file_hash 计算文件哈希");

    let mut file = fs::File::open(path).map_err(|e| {
        error!(file = %path.display(), error = %e, "无法打开文件");
        CctError::ParseFileRead(format!("{}: {e}", path.display()))
    })?;

    let mut hasher = Sha256::new();
    let mut buffer = [0u8; 8192];

    loop {
        let bytes_read = file.read(&mut buffer).map_err(|e| {
            error!(file = %path.display(), error = %e, "读取文件失败");
            CctError::ParseFileRead(format!("{}: {e}", path.display()))
        })?;

        if bytes_read == 0 {
            break;
        }
        hasher.update(&buffer[..bytes_read]);
    }

    let hash = format!("{:x}", hasher.finalize());
    debug!(file = %path.display(), hash = %hash, "哈希计算完成");
    Ok(hash)
}
