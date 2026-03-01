use thiserror::Error;

/// 统一错误类型 — 涵盖 doc/01~09 所有错误码
#[derive(Debug, Error)]
pub enum CctError {
    // ── 项目管理 (doc/01 §6) ──
    #[error("项目不存在: {0}")]
    ProjectNotFound(String),
    #[error("项目名称已存在: {0}")]
    ProjectNameExists(String),
    #[error("源码目录无效: {0}")]
    InvalidSourceRoot(String),
    #[error("编译数据库格式无效: {0}")]
    InvalidCompileDb(String),
    #[error("编译数据库未找到: {0}")]
    CompileDbNotFound(String),

    // ── SSH 相关 (doc/01 §6) ──
    #[error("SSH 连接失败: {0}")]
    SshConnectionFailed(String),
    #[error("SSH 认证失败: {0}")]
    SshAuthFailed(String),
    #[error("SSH 连接超时")]
    SshTimeout,
    #[error("SSH Host Key 验证失败: {0}")]
    SshHostKeyVerifyFailed(String),
    #[error("SFTP 操作失败: {0}")]
    SftpError(String),

    // ── Agent 相关 (doc/01 §6) ──
    #[error("Agent 部署失败: {0}")]
    AgentDeployFailed(String),
    #[error("Agent 启动失败: {0}")]
    AgentStartFailed(String),
    #[error("Agent 不兼容: 服务器架构 {0} 不支持")]
    AgentIncompatibleArch(String),
    #[error("Agent 版本不匹配: 已安装 {installed}, 需要 {required}")]
    AgentVersionMismatch { installed: String, required: String },

    // ── 解析相关 (doc/02 §9) ──
    #[error("源码目录为空或无 C/C++ 文件")]
    ParseNoSource,
    #[error("Clang LibTooling 初始化失败: {0}")]
    ParseClangInit(String),
    #[error("文件读取失败: {0}")]
    ParseFileRead(String),
    #[error("语法解析错误: {0}")]
    ParseSyntax(String),
    #[error("文件编码不支持: {0}")]
    ParseEncoding(String),
    #[error("解析内存不足")]
    ParseOutOfMemory,
    #[error("解析被用户取消")]
    ParseCancelled,
    #[error("索引写入失败: {0}")]
    IndexWrite(String),
    #[error("索引数据损坏: {0}")]
    IndexCorrupt(String),

    // ── 远程解析 (doc/02 §9) ──
    #[error("远程 Agent 不可达")]
    RemoteAgentUnreachable,
    #[error("远程 Agent 解析失败: {0}")]
    RemoteAgentParseFailed(String),
    #[error("索引数据传输失败: {0}")]
    RemoteTransferFailed(String),
    #[error("索引数据传输中断")]
    RemoteTransferInterrupted,
    #[error("索引数据解压失败: {0}")]
    RemoteDecompressFailed(String),

    // ── 查询相关 (doc/03) ──
    #[error("符号未找到: {0}")]
    SymbolNotFound(String),
    #[error("查询超时")]
    QueryTimeout,

    // ── AI 相关 (doc/07) ──
    #[error("AI 连接失败: {0}")]
    AiConnectionFailed(String),
    #[error("AI 配置无效: {0}")]
    AiConfigInvalid(String),

    // ── 通用 ──
    #[error("配置加载失败: {0}")]
    ConfigLoad(String),
    #[error("配置保存失败: {0}")]
    ConfigSave(String),
    #[error("数据库错误: {0}")]
    Database(String),
    #[error("IO 错误: {0}")]
    Io(#[from] std::io::Error),
    #[error("序列化错误: {0}")]
    Serialization(#[from] serde_json::Error),
    #[error("内部错误: {0}")]
    Internal(String),
}

impl serde::Serialize for CctError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}
