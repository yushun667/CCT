# CCT 构建与发布

## 编译产物是否包含所有运行需要的东西

**结论**：CI/Release 构建**已包含完整功能**（含 LLVM/Clang 解析引擎），安装包可直接用于本地 C/C++ 源码解析与调用图分析。

| 内容 | 是否包含 | 说明 |
|------|----------|------|
| 应用本体（前端 + 后端二进制） | ✅ | 安装即用 |
| 系统 WebView 依赖 | ✅ | 各平台安装包会处理或声明依赖 |
| **LLVM/Clang 解析引擎** | ✅ | CI 中安装 LLVM 18 并链接，本地解析可用 |
| 远程 cct-agent | ❌ | 需从 Release 的 Agent 构建产物单独下载并部署到远程服务器 |

**本地开发**：若未安装 LLVM，可设置 `LLVM_PREFIX` 或使用 `third_party/llvm/`，再执行 `npm run tauri build`。

---

## 三平台安装包（GitHub Actions）

推送 **tag**（格式 `v*`，如 `v0.1.0`）到 GitHub 后，会自动触发 [Release 工作流](.github/workflows/release.yml)，在 **Windows、macOS、Linux** 上构建并生成安装包。

### 产出的安装包

| 平台     | 产物说明 |
|----------|----------|
| Windows  | `.msi`（安装向导）、`.exe`（NSIS 安装程序） |
| macOS    | `.dmg`（磁盘镜像）、`.app`（应用包）；支持 x64 与 arm64 |
| Linux    | `.deb`（Debian/Ubuntu）、`.AppImage`（通用） |

### 触发方式

```bash
git tag v0.1.0
git push origin v0.1.0
```

构建完成后，在 GitHub 仓库的 **Releases** 中会出现草稿发布，包含各平台安装包，可编辑说明后发布。

### 本地单平台构建

在当前系统上构建当前平台安装包：

```bash
npm ci
npm run tauri build
```

产物在 `src-tauri/target/release/bundle/` 下（或对应 `target/<triple>/release/bundle/`）。

## cct-agent（远程解析）

推送 tag 时还会触发 [Agent Release 工作流](.github/workflows/agent-release.yml)，构建可在 Linux（x86_64 / aarch64）上运行的 **cct-agent** 二进制，用于远程项目解析。产物以 workflow 的 artifacts 形式提供。
