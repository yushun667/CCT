//! SSH 连接管理 — 封装 SSH 连接的建立、维护与断开
//!
//! 当前为占位实现，API 形状已按需求定义。
//! 后续可替换为 russh 或其它 SSH 库的实际连接逻辑。

use cct_core::error::CctError;
use cct_core::models::project::SSHConfig;
use tracing::{debug, info, warn};

/// SSH 连接状态
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionState {
    Disconnected,
    Connecting,
    Connected,
    Error,
}

/// SSH 连接封装
///
/// # 设计说明（外观模式）
/// 将底层 SSH 协议细节隐藏在统一接口后面，
/// 上层只需调用 `connect` / `disconnect` / `is_connected` 即可。
pub struct SshConnection {
    state: ConnectionState,
    config: SSHConfig,
}

impl SshConnection {
    /// 建立 SSH 连接
    ///
    /// # 参数
    /// - `config`: SSH 连接配置，包含主机、端口、认证方式等
    ///
    /// # 返回
    /// - `Ok(Self)`: 连接成功（当前为占位，始终返回成功）
    /// - `Err(CctError)`: 连接失败
    pub async fn connect(config: &SSHConfig) -> Result<Self, CctError> {
        info!(
            host = %config.host,
            port = config.port,
            username = %config.username,
            "SshConnection::connect — 尝试建立 SSH 连接"
        );

        // 占位实现：验证配置的基本合理性
        if config.host.is_empty() {
            warn!("SSH 主机地址为空");
            return Err(CctError::SshConnectionFailed(
                "主机地址不能为空".to_string(),
            ));
        }
        if config.username.is_empty() {
            warn!("SSH 用户名为空");
            return Err(CctError::SshAuthFailed("用户名不能为空".to_string()));
        }

        debug!(
            host = %config.host,
            port = config.port,
            "占位实现：模拟 SSH 连接成功"
        );

        Ok(Self {
            state: ConnectionState::Connected,
            config: config.clone(),
        })
    }

    /// 断开 SSH 连接
    pub async fn disconnect(&mut self) -> Result<(), CctError> {
        info!(
            host = %self.config.host,
            "SshConnection::disconnect — 断开 SSH 连接"
        );
        self.state = ConnectionState::Disconnected;
        debug!("SSH 连接已断开");
        Ok(())
    }

    /// 检查连接是否活跃
    pub fn is_connected(&self) -> bool {
        let connected = self.state == ConnectionState::Connected;
        debug!(
            host = %self.config.host,
            connected,
            "SshConnection::is_connected"
        );
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
}
