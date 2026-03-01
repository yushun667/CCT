//! SSH 连接管理 — 基于 russh 实现真实 SSH 连接
//!
//! 封装 SSH 协议的连接建立、认证、通道管理与命令执行。
//! 支持密钥认证、密码认证和 SSH Agent 认证三种方式。

use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use cct_core::error::CctError;
use cct_core::models::project::{HostKeyPolicy, SSHAuthMethod, SSHConfig};
use russh::client::{self, Handle, Msg};
use russh::{Channel, ChannelMsg, Disconnect};
use tracing::{debug, error, info, warn};

/// SSH 连接状态
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionState {
    Disconnected,
    Connecting,
    Connected,
    Error,
}

/// SSH 客户端事件处理器
///
/// 实现 `russh::client::Handler`，处理 SSH 协议层的服务器回调。
/// 当前主要处理服务器密钥验证策略。
struct ClientHandler {
    known_hosts_policy: HostKeyPolicy,
}

#[async_trait]
impl client::Handler for ClientHandler {
    type Error = russh::Error;

    async fn check_server_key(
        &mut self,
        _server_public_key: &russh_keys::key::PublicKey,
    ) -> Result<bool, Self::Error> {
        info!("ClientHandler::check_server_key — 验证服务器密钥");
        match self.known_hosts_policy {
            HostKeyPolicy::Accept => {
                warn!("接受服务器密钥（策略: Accept）");
                Ok(true)
            }
            HostKeyPolicy::Reject => {
                warn!("拒绝未知服务器密钥（策略: Reject）");
                Ok(false)
            }
            HostKeyPolicy::AskUser => {
                warn!("服务器密钥验证暂自动接受（策略: AskUser，交互提示待实现）");
                Ok(true)
            }
        }
    }
}

/// SSH 连接封装 — 基于 russh 的真实 SSH 连接
///
/// # 设计说明（外观模式）
/// 将底层 russh SSH 协议细节隐藏在统一接口后面，
/// 上层只需调用 `connect` / `disconnect` / `exec_command` 等方法。
pub struct SshConnection {
    state: ConnectionState,
    config: SSHConfig,
    handle: Handle<ClientHandler>,
}

impl SshConnection {
    /// 建立 SSH 连接并完成认证
    ///
    /// # 参数
    /// - `config`: SSH 连接配置，包含主机、端口、认证方式等
    ///
    /// # 返回
    /// - `Ok(Self)`: 认证成功的 SSH 连接
    /// - `Err(CctError)`: 连接或认证失败
    pub async fn connect(config: &SSHConfig) -> Result<Self, CctError> {
        info!(
            host = %config.host, port = config.port, username = %config.username,
            "SshConnection::connect — 尝试建立 SSH 连接"
        );

        if config.host.is_empty() {
            warn!("SSH 主机地址为空");
            return Err(CctError::SshConnectionFailed("主机地址不能为空".into()));
        }
        if config.username.is_empty() {
            warn!("SSH 用户名为空");
            return Err(CctError::SshAuthFailed("用户名不能为空".into()));
        }

        let client_config = client::Config {
            inactivity_timeout: Some(Duration::from_secs(
                config.keep_alive_interval.max(10) as u64,
            )),
            ..Default::default()
        };

        let handler = ClientHandler {
            known_hosts_policy: config.known_hosts_policy.clone(),
        };

        debug!(host = %config.host, port = config.port, "正在建立 TCP 连接并协商 SSH 协议...");

        let addr = (config.host.as_str(), config.port);
        let timeout = Duration::from_secs(config.connect_timeout.max(5) as u64);

        let mut session = tokio::time::timeout(
            timeout,
            client::connect(Arc::new(client_config), addr, handler),
        )
        .await
        .map_err(|_| {
            error!(host = %config.host, timeout_secs = timeout.as_secs(), "SSH 连接超时");
            CctError::SshTimeout
        })?
        .map_err(|e| {
            error!(host = %config.host, error = %e, "SSH 握手失败");
            CctError::SshConnectionFailed(format!("SSH 握手失败: {e}"))
        })?;

        info!(host = %config.host, "SSH 握手完成，开始认证...");
        Self::authenticate(&mut session, config).await?;
        info!(host = %config.host, "SSH 认证成功，连接已建立");

        Ok(Self {
            state: ConnectionState::Connected,
            config: config.clone(),
            handle: session,
        })
    }

    /// 执行认证流程（密钥 / 密码 / Agent）
    async fn authenticate(
        session: &mut Handle<ClientHandler>,
        config: &SSHConfig,
    ) -> Result<(), CctError> {
        info!("SshConnection::authenticate — 开始认证流程");
        match &config.auth_method {
            SSHAuthMethod::Key {
                key_path,
                passphrase_ref,
            } => {
                info!(key_path = %key_path, "使用密钥文件认证");
                let key_pair = russh_keys::load_secret_key(
                    std::path::Path::new(key_path),
                    passphrase_ref.as_deref(),
                )
                .map_err(|e| {
                    error!(key_path = %key_path, error = %e, "密钥文件加载失败");
                    CctError::SshAuthFailed(format!("密钥加载失败: {e}"))
                })?;

                let auth_ok = session
                    .authenticate_publickey(&config.username, Arc::new(key_pair))
                    .await
                    .map_err(|e| {
                        error!(error = %e, "公钥认证协议错误");
                        CctError::SshAuthFailed(format!("公钥认证失败: {e}"))
                    })?;

                if !auth_ok {
                    return Err(CctError::SshAuthFailed("公钥认证被服务器拒绝".into()));
                }
                debug!("密钥认证成功");
            }

            SSHAuthMethod::Password { password_ref } => {
                info!("使用密码认证");
                let auth_ok = session
                    .authenticate_password(&config.username, password_ref)
                    .await
                    .map_err(|e| {
                        error!(error = %e, "密码认证协议错误");
                        CctError::SshAuthFailed(format!("密码认证失败: {e}"))
                    })?;

                if !auth_ok {
                    return Err(CctError::SshAuthFailed("密码认证被服务器拒绝".into()));
                }
                debug!("密码认证成功");
            }

            SSHAuthMethod::Agent => {
                info!("使用 SSH Agent 认证");
                Self::authenticate_with_agent(session, &config.username).await?;
                debug!("Agent 认证成功");
            }
        }

        Ok(())
    }

    /// 通过 SSH Agent 进行认证（Unix/macOS）
    #[cfg(unix)]
    async fn authenticate_with_agent(
        session: &mut Handle<ClientHandler>,
        username: &str,
    ) -> Result<(), CctError> {
        info!("SshConnection::authenticate_with_agent — 连接 SSH Agent");
        let agent_sock = std::env::var("SSH_AUTH_SOCK").map_err(|_| {
            CctError::SshAuthFailed("SSH_AUTH_SOCK 环境变量未设置，无法连接 SSH Agent".into())
        })?;

        debug!(socket = %agent_sock, "连接 SSH Agent...");

        let stream = tokio::net::UnixStream::connect(&agent_sock)
            .await
            .map_err(|e| {
                error!(socket = %agent_sock, error = %e, "无法连接 SSH Agent");
                CctError::SshAuthFailed(format!("连接 SSH Agent 失败: {e}"))
            })?;

        let mut agent = russh_keys::agent::client::AgentClient::connect(stream);
        let identities = agent.request_identities().await.map_err(|e| {
            error!(error = %e, "请求 Agent 身份列表失败");
            CctError::SshAuthFailed(format!("Agent 身份列表获取失败: {e}"))
        })?;

        if identities.is_empty() {
            return Err(CctError::SshAuthFailed("SSH Agent 中无可用密钥".into()));
        }

        info!(count = identities.len(), "从 Agent 获取到密钥");

        for identity in identities {
            let (returned_agent, result) =
                session.authenticate_future(username, identity, agent).await;
            agent = returned_agent;
            match result {
                Ok(true) => {
                    info!("Agent 密钥认证成功");
                    return Ok(());
                }
                Ok(false) => {
                    debug!("Agent 密钥被服务器拒绝，尝试下一个");
                    continue;
                }
                Err(_e) => {
                    debug!("Agent 密钥认证出错，尝试下一个");
                    continue;
                }
            }
        }

        Err(CctError::SshAuthFailed(
            "Agent 中所有密钥均被服务器拒绝".into(),
        ))
    }

    /// Windows 平台不支持 SSH Agent
    #[cfg(not(unix))]
    async fn authenticate_with_agent(
        _session: &mut Handle<ClientHandler>,
        _username: &str,
    ) -> Result<(), CctError> {
        Err(CctError::SshAuthFailed(
            "SSH Agent 认证仅支持 Unix/macOS 平台".into(),
        ))
    }

    /// 断开 SSH 连接
    pub async fn disconnect(&mut self) -> Result<(), CctError> {
        info!(host = %self.config.host, "SshConnection::disconnect — 断开 SSH 连接");
        self.handle
            .disconnect(Disconnect::ByApplication, "", "")
            .await
            .map_err(|e| {
                error!(error = %e, "断开连接时出错");
                CctError::SshConnectionFailed(format!("断开连接失败: {e}"))
            })?;
        self.state = ConnectionState::Disconnected;
        debug!(host = %self.config.host, "SSH 连接已断开");
        Ok(())
    }

    /// 检查连接是否活跃
    pub fn is_connected(&self) -> bool {
        let connected = self.state == ConnectionState::Connected;
        debug!(host = %self.config.host, connected, "SshConnection::is_connected");
        connected
    }

    /// 获取当前连接状态
    pub fn state(&self) -> ConnectionState {
        self.state
    }

    /// 获取连接使用的配置引用
    pub fn config(&self) -> &SSHConfig {
        &self.config
    }

    /// 打开新的 SSH 会话通道
    ///
    /// # 返回
    /// 可用于执行命令或请求子系统的 SSH 通道
    pub async fn open_channel(&self) -> Result<Channel<Msg>, CctError> {
        info!(host = %self.config.host, "SshConnection::open_channel — 打开 SSH 通道");
        self.handle.channel_open_session().await.map_err(|e| {
            error!(error = %e, "打开 SSH 通道失败");
            CctError::SshConnectionFailed(format!("打开 SSH 通道失败: {e}"))
        })
    }

    /// 执行远程命令并返回标准输出
    ///
    /// # 参数
    /// - `cmd`: 要执行的命令字符串
    ///
    /// # 返回
    /// 命令的标准输出内容（UTF-8 字符串）
    pub async fn exec_command(&self, cmd: &str) -> Result<String, CctError> {
        info!(host = %self.config.host, cmd = %cmd, "SshConnection::exec_command — 执行远程命令");

        let mut channel = self.open_channel().await?;
        channel.exec(true, cmd).await.map_err(|e| {
            error!(cmd = %cmd, error = %e, "发送 exec 请求失败");
            CctError::SshConnectionFailed(format!("执行命令失败: {e}"))
        })?;

        let mut output = Vec::new();
        let mut exit_code: Option<u32> = None;

        loop {
            let Some(msg) = channel.wait().await else {
                break;
            };
            match msg {
                ChannelMsg::Data { ref data } => {
                    output.extend_from_slice(data);
                }
                ChannelMsg::ExitStatus { exit_status } => {
                    exit_code = Some(exit_status);
                    debug!(cmd = %cmd, exit_status, "命令返回退出码");
                }
                _ => {}
            }
        }

        if let Some(code) = exit_code {
            if code != 0 {
                warn!(cmd = %cmd, exit_code = code, "命令退出码非零");
            }
        }

        let result = String::from_utf8(output).map_err(|e| {
            CctError::SshConnectionFailed(format!("命令输出编码错误: {e}"))
        })?;

        debug!(cmd = %cmd, output_len = result.len(), "命令执行完毕");
        Ok(result)
    }
}
