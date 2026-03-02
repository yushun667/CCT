#!/usr/bin/env bash
# ─────────────────────────────────────────────────
#  CCT 一键启动脚本
#  用法:  ./run.sh          开发模式（热重载）
#         ./run.sh build    构建生产包
#         ./run.sh release  构建 Release 优化包
# ─────────────────────────────────────────────────
set -euo pipefail

PROJECT_DIR="$(cd "$(dirname "$0")" && pwd)"
cd "$PROJECT_DIR"

# macOS: 让最终二进制能找到 third_party/llvm 下的 libunwind/libc++ 等动态库（run.sh 调用时路径始终正确）
if [[ "$(uname -s)" == "Darwin" ]]; then
    export RUSTFLAGS="${RUSTFLAGS:-} -C link-arg=-Wl,-rpath,$PROJECT_DIR/third_party/llvm/lib"
fi

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
CYAN='\033[0;36m'
NC='\033[0m'

log()  { echo -e "${CYAN}[CCT]${NC} $*"; }
ok()   { echo -e "${GREEN}[CCT]${NC} $*"; }
warn() { echo -e "${YELLOW}[CCT]${NC} $*"; }
err()  { echo -e "${RED}[CCT]${NC} $*" >&2; }

# ── 前置检查 ────────────────────────────────────
check_deps() {
    local missing=0

    if ! command -v node &>/dev/null; then
        err "未找到 node，请先安装 Node.js (>=18)"
        missing=1
    fi

    if ! command -v cargo &>/dev/null; then
        err "未找到 cargo，请先安装 Rust toolchain"
        missing=1
    fi

    if ! command -v npm &>/dev/null; then
        err "未找到 npm"
        missing=1
    fi

    if [ ! -d "$PROJECT_DIR/third_party/llvm/lib" ]; then
        err "未找到 LLVM 库: third_party/llvm/lib"
        err "请先下载预编译 LLVM 到 third_party/llvm/"
        missing=1
    fi

    if [ $missing -ne 0 ]; then
        exit 1
    fi
}

# ── 安装前端依赖 ────────────────────────────────
ensure_node_modules() {
    if [ ! -d "$PROJECT_DIR/node_modules" ]; then
        log "首次运行，安装前端依赖..."
        npm install
        ok "前端依赖安装完成"
    fi
}

# ── 开发模式 ────────────────────────────────────
cmd_dev() {
    check_deps
    ensure_node_modules

    log "启动开发模式（前端热重载 + Rust 后端）..."
    log "前端: http://localhost:1420"
    log "按 Ctrl+C 停止"
    echo ""

    npx tauri dev
}

# ── 构建 ────────────────────────────────────────
cmd_build() {
    check_deps
    ensure_node_modules

    log "构建生产版本..."
    npx tauri build
    ok "构建完成！产物在 /tmp/cct-target/release/bundle/"
}

# ── Release 构建 ─────────────────────────────────
cmd_release() {
    check_deps
    ensure_node_modules

    log "构建 Release 优化版本..."
    npx tauri build --release
    ok "构建完成！产物在 /tmp/cct-target/release/bundle/"
}

# ── 入口 ────────────────────────────────────────
case "${1:-dev}" in
    dev)
        cmd_dev
        ;;
    build)
        cmd_build
        ;;
    release)
        cmd_release
        ;;
    *)
        echo "用法: $0 {dev|build|release}"
        echo ""
        echo "  dev      开发模式，前端热重载（默认）"
        echo "  build    构建生产版本"
        echo "  release  构建 Release 优化版本"
        exit 1
        ;;
esac
