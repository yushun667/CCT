//! SFTP 客户端 — 基于 russh-sftp 实现真实远程文件系统操作
//!
//! 提供远程目录浏览、文件读取和上传功能，
//! 通过 SSH 通道上的 SFTP 子系统与远程服务器通信。

use std::path::Path;

use cct_core::error::CctError;
use serde::{Deserialize, Serialize};
use tracing::{debug, error, info, warn};

use crate::connection::SshConnection;

/// 远程文件条目
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemoteFileEntry {
    /// 文件/目录名
    pub name: String,
    /// 完整远程路径
    pub path: String,
    /// 是否为目录
    pub is_dir: bool,
    /// 文件大小（字节），目录为 0
    pub size: u64,
    /// 最后修改时间（Unix 时间戳）
    pub modified: u64,
}

/// SFTP 客户端 — 通过 SSH SFTP 子系统操作远程文件
///
/// # 设计说明（代理模式）
/// 作为远程文件系统的本地代理，将文件操作请求
/// 通过 SFTP 协议转发到远程服务器并返回结果。
pub struct SftpClient {
    sftp: russh_sftp::client::SftpSession,
    host: String,
}

impl SftpClient {
    /// 从已建立的 SSH 连接创建 SFTP 客户端
    ///
    /// # 参数
    /// - `conn`: 已认证的 SSH 连接
    ///
    /// # 返回
    /// - `Ok(Self)`: SFTP 会话就绪
    /// - `Err(CctError)`: SFTP 子系统初始化失败
    pub async fn from_connection(conn: &SshConnection) -> Result<Self, CctError> {
        info!(
            host = %conn.config().host,
            "SftpClient::from_connection — 初始化 SFTP 会话"
        );

        let channel = conn.open_channel().await?;

        debug!(host = %conn.config().host, "请求 SFTP 子系统...");
        channel
            .request_subsystem(true, "sftp")
            .await
            .map_err(|e| {
                error!(error = %e, "请求 SFTP 子系统失败");
                CctError::SftpError(format!("请求 SFTP 子系统失败: {e}"))
            })?;

        let sftp = russh_sftp::client::SftpSession::new(channel.into_stream())
            .await
            .map_err(|e| {
                error!(error = %e, "SFTP 会话初始化失败");
                CctError::SftpError(format!("SFTP 会话初始化失败: {e}"))
            })?;

        info!(host = %conn.config().host, "SFTP 会话已建立");

        Ok(Self {
            sftp,
            host: conn.config().host.clone(),
        })
    }

    /// 列出远程目录内容
    ///
    /// # 参数
    /// - `path`: 远程目录的绝对路径
    ///
    /// # 返回
    /// - `Ok(Vec<RemoteFileEntry>)`: 目录条目列表（排除 `.` 和 `..`）
    /// - `Err(CctError)`: SFTP 操作失败
    pub async fn list_directory(&self, path: &str) -> Result<Vec<RemoteFileEntry>, CctError> {
        info!(
            host = %self.host, path = %path,
            "SftpClient::list_directory — 列出远程目录"
        );

        let entries = self.sftp.read_dir(path).await.map_err(|e| {
            error!(path = %path, error = %e, "SFTP read_dir 失败");
            CctError::SftpError(format!("列目录失败 ({path}): {e}"))
        })?;

        let mut result = Vec::new();
        for entry in entries {
            let name = entry.file_name();
            if name == "." || name == ".." {
                continue;
            }

            let is_dir = entry
                .file_type()
                .is_dir();
            let size = entry.metadata().size.unwrap_or(0);
            let modified = entry.metadata().mtime.unwrap_or(0) as u64;

            result.push(RemoteFileEntry {
                name: name.to_string(),
                path: format!("{}/{}", path.trim_end_matches('/'), name),
                is_dir,
                size,
                modified,
            });
        }

        debug!(
            path = %path, count = result.len(),
            "目录列表完成"
        );
        Ok(result)
    }

    /// 读取远程文件内容
    ///
    /// # 参数
    /// - `path`: 远程文件的绝对路径
    ///
    /// # 返回
    /// - `Ok(Vec<u8>)`: 文件内容的字节流
    /// - `Err(CctError)`: 读取失败
    pub async fn read_file(&self, path: &str) -> Result<Vec<u8>, CctError> {
        info!(
            host = %self.host, path = %path,
            "SftpClient::read_file — 读取远程文件"
        );

        let data = self.sftp.read(path).await.map_err(|e| {
            error!(path = %path, error = %e, "SFTP 文件读取失败");
            CctError::SftpError(format!("读取文件失败 ({path}): {e}"))
        })?;

        debug!(path = %path, size = data.len(), "文件读取完成");
        Ok(data)
    }

    /// 上传本地文件到远程
    ///
    /// # 参数
    /// - `local_path`: 本地文件路径
    /// - `remote_path`: 远程目标路径
    ///
    /// # 返回
    /// - `Ok(())`: 上传成功
    /// - `Err(CctError)`: 上传失败
    pub async fn upload_file(
        &self,
        local_path: &Path,
        remote_path: &str,
    ) -> Result<(), CctError> {
        info!(
            host = %self.host,
            local = %local_path.display(),
            remote = %remote_path,
            "SftpClient::upload_file — 上传文件到远程"
        );

        if !local_path.exists() {
            warn!(path = %local_path.display(), "本地文件不存在");
            return Err(CctError::SftpError(format!(
                "本地文件不存在: {}",
                local_path.display()
            )));
        }

        let data = std::fs::read(local_path).map_err(|e| {
            error!(path = %local_path.display(), error = %e, "读取本地文件失败");
            CctError::SftpError(format!("读取本地文件失败: {e}"))
        })?;

        debug!(
            remote = %remote_path, size = data.len(),
            "正在上传文件..."
        );

        self.sftp.write(remote_path, &data).await.map_err(|e| {
            error!(remote = %remote_path, error = %e, "SFTP 文件写入失败");
            CctError::SftpError(format!("上传文件失败 ({remote_path}): {e}"))
        })?;

        info!(
            remote = %remote_path, size = data.len(),
            "文件上传完成"
        );
        Ok(())
    }

    /// 检查远程路径是否存在
    ///
    /// # 参数
    /// - `path`: 远程路径
    ///
    /// # 返回
    /// 路径是否存在
    pub async fn exists(&self, path: &str) -> Result<bool, CctError> {
        debug!(host = %self.host, path = %path, "SftpClient::exists — 检查远程路径");
        self.sftp.try_exists(path).await.map_err(|e| {
            CctError::SftpError(format!("检查路径存在失败 ({path}): {e}"))
        })
    }
}
