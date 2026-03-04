/// build.rs — 编译 C++ LibTooling 桥接代码并链接 LLVM/Clang 库
///
/// 使用 cc crate 编译 cpp/src/bridge.cpp，
/// 配置 LLVM/Clang 的 include/lib 路径，
/// 链接所需的静态库和系统库。

use std::env;
use std::path::PathBuf;
use std::process::Command;

fn main() {
    println!("cargo::rustc-check-cfg=cfg(no_clang_bridge)");
    let manifest_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    let manifest_path = PathBuf::from(&manifest_dir);
    let workspace_root = manifest_path
        .parent().unwrap()
        .parent().unwrap()
        .to_path_buf();

    let llvm_config_exe = if cfg!(windows) { "llvm-config.exe" } else { "llvm-config" };
    // 1) 优先使用 third_party/llvm
    let third_party_llvm = workspace_root.join("third_party").join("llvm");
    let llvm_config_third = third_party_llvm.join("bin").join(llvm_config_exe);

    let (llvm_root, llvm_config, use_static) = if llvm_config_third.exists() {
        (third_party_llvm.clone(), llvm_config_third, true)
    } else if let Ok(prefix) = env::var("LLVM_PREFIX") {
        let root = PathBuf::from(&prefix);
        let config = root.join("bin").join(llvm_config_exe);
        if config.exists() {
            (root, config, false)
        } else {
            println!("cargo:warning=LLVM_PREFIX={:?} but {} not found", prefix, llvm_config_exe);
            println!("cargo:rustc-cfg=no_clang_bridge");
            return;
        }
    } else if let Ok(path) = which_llvm_config_prefix() {
        let root = PathBuf::from(&path);
        let config = root.join("bin").join(llvm_config_exe);
        if config.exists() {
            (root, config, false)
        } else {
            println!("cargo:warning=llvm-config --prefix={:?} but bin/{} missing", path, llvm_config_exe);
            println!("cargo:rustc-cfg=no_clang_bridge");
            return;
        }
    } else {
        println!("cargo:warning=LLVM not found (no third_party/llvm, no LLVM_PREFIX, no llvm-config in PATH), skipping C++ bridge");
        println!("cargo:rustc-cfg=no_clang_bridge");
        return;
    };

    let llvm_include = llvm_root.join("include");
    let llvm_lib = llvm_root.join("lib");
    let cpp_include = PathBuf::from(&manifest_dir).join("cpp").join("include");
    let cpp_src = PathBuf::from(&manifest_dir).join("cpp").join("src").join("bridge.cpp");

    // Get LLVM C++ flags
    let cxxflags = run_llvm_config(&llvm_config, &["--cxxflags"]);

    // Compile C++ source
    let mut build = cc::Build::new();
    build
        .cpp(true)
        .file(&cpp_src)
        .include(&llvm_include)
        .include(&cpp_include)
        .flag("-std=c++17")
        .flag("-fno-exceptions")
        .flag("-fno-rtti")
        .flag("-D__STDC_CONSTANT_MACROS")
        .flag("-D__STDC_FORMAT_MACROS")
        .flag("-D__STDC_LIMIT_MACROS")
        .warnings(false);

    // Add extra include flags from llvm-config
    for flag in cxxflags.split_whitespace() {
        if flag.starts_with("-I") {
            build.flag(flag);
        }
    }

    build.compile("cct_bridge");

    // Link LLVM/Clang libraries
    println!("cargo:rustc-link-search=native={}", llvm_lib.display());

    // Clang libraries (order matters for static linking)
    let clang_libs = [
        "clangTooling",
        "clangToolingCore",
        "clangFrontend",
        "clangFrontendTool",
        "clangDriver",
        "clangSerialization",
        "clangCodeGen",
        "clangParse",
        "clangSema",
        "clangAnalysis",
        "clangAnalysisFlowSensitive",
        "clangEdit",
        "clangAST",
        "clangASTMatchers",
        "clangLex",
        "clangBasic",
        "clangRewrite",
        "clangRewriteFrontend",
        "clangIndex",
        "clangAPINotes",
        "clangSupport",
    ];

    let link_type = if use_static { "static" } else { "dylib" };
    for lib in &clang_libs {
        println!("cargo:rustc-link-lib={}={}", link_type, lib);
    }

    // LLVM libraries from llvm-config
    let llvm_libs_str = run_llvm_config(&llvm_config, &["--libs", "core", "support", "option",
        "frontendopenmp", "frontendhlsl", "mc", "mcparser", "bitreader",
        "profiledata", "binaryformat", "targetparser", "remarks",
        "bitstreamreader", "demangle", "textapi",
        "windowsdriver", "windowsmanifest"]);

    for lib in llvm_libs_str.split_whitespace() {
        if let Some(name) = lib.strip_prefix("-l") {
            println!("cargo:rustc-link-lib={}={}", link_type, name);
        }
    }

    // System libraries
    let system_libs = run_llvm_config(&llvm_config, &["--system-libs"]);
    for lib in system_libs.split_whitespace() {
        if let Some(name) = lib.strip_prefix("-l") {
            println!("cargo:rustc-link-lib=dylib={}", name);
        }
    }

    // macOS: link system library search paths (e.g. Homebrew)
    if cfg!(target_os = "macos") {
        println!("cargo:rustc-link-lib=c++");
        if let Ok(output) = std::process::Command::new("brew")
            .args(["--prefix", "zstd"])
            .output()
        {
            let prefix = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !prefix.is_empty() {
                println!("cargo:rustc-link-search=native={}/lib", prefix);
            }
        }
    } else {
        println!("cargo:rustc-link-lib=stdc++");
    }

    // Add LLVM lib to rpath for dynamic dependencies (libunwind etc.) — Unix only
    #[cfg(unix)]
    println!("cargo:rustc-link-arg=-Wl,-rpath,{}", llvm_lib.display());

    // Rerun if C++ sources change
    println!("cargo:rerun-if-changed=cpp/src/bridge.cpp");
    println!("cargo:rerun-if-changed=cpp/include/bridge.h");
}

/// 若 PATH 中有 llvm-config 或 llvm-config-18，返回其 --prefix
fn which_llvm_config_prefix() -> Result<String, ()> {
    for name in &["llvm-config-18", "llvm-config"] {
        if let Ok(output) = Command::new(name).arg("--prefix").output() {
            if output.status.success() {
                let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
                if !path.is_empty() {
                    return Ok(path);
                }
            }
        }
    }
    Err(())
}

fn run_llvm_config(llvm_config: &PathBuf, args: &[&str]) -> String {
    let output = Command::new(llvm_config)
        .args(args)
        .output()
        .unwrap_or_else(|e| panic!("Failed to run llvm-config: {}", e));

    String::from_utf8(output.stdout)
        .unwrap_or_default()
        .trim()
        .to_string()
}
