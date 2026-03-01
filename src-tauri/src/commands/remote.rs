//! 远程项目 Tauri 命令 — SSH 连接、Agent 部署与远程文件浏览
//!
//! 提供前端调用的远程项目管理接口，基于 cct-ssh 实现真实 SSH 连接。

use cct_core::error::CctError;
use cct_core::models::project::{SSHAuthMethod, SSHConfig};
use cct_ssh::{AgentRpcClient, SftpClient, SshConnection};
use tracing::{debug, error, info, warn};

/// 测试 SSH 连接
///
/// 创建临时 SSH 配置并尝试建立连接，验证 SSH 可达性。
/// 使用 Agent 认证方式进行测试。
///
/// # 参数
/// - `host`: SSH 主机地址
/// - `port`: SSH 端口
/// - `username`: 用户名
///
/// # 返回
/// - `Ok(true)`: 连接测试成功
/// - `Ok(false)`: 连接测试失败
/// - `Err(CctError)`: 参数校验失败
#[tauri::command]
pub async fn test_ssh_connection(
    host: String,
    port: u16,
    username: String,
) -> Result<bool, CctError> {
    info!(
        host = %host, port = port, username = %username,
        "Tauri Command: test_ssh_connection"
    );

    if host.is_empty() {
        warn!("主机地址为空");
        return Ok(false);
    }
    if username.is_empty() {
        warn!("用户名为空");
        return Ok(false);
    }

    let config = SSHConfig {
        host: host.clone(),
        port,
        username: username.clone(),
        auth_method: SSHAuthMethod::Agent,
        ..Default::default()
    };

    match SshConnection::connect(&config).await {
        Ok(mut conn) => {
            info!(host = %host, "SSH 连接测试成功");
            let _ = conn.disconnect().await;
            Ok(true)
        }
        Err(e) => {
            warn!(host = %host, error = %e, "SSH 连接测试失败");
            Ok(false)
        }
    }
}

/// 浏览远程目录
///
/// 通过 SFTP 列出远程服务器上指定目录的内容。
///
/// # 参数
/// - `project_id`: 项目 UUID（用于获取 SSH 配置）
/// - `path`: 远程目录路径
///
/// # 返回
/// 目录条目列表（JSON 数组）
#[tauri::command]
pub async fn browse_remote_dir(
    project_id: String,
    path: String,
) -> Result<Vec<serde_json::Value>, CctError> {
    info!(
        project_id = %project_id, path = %path,
        "Tauri Command: browse_remote_dir"
    );

    let ssh_config = load_ssh_config(&project_id).await?;

    let conn = SshConnection::connect(&ssh_config).await?;
    let sftp = SftpClient::from_connection(&conn).await?;

    let entries = sftp.list_directory(&path).await?;

    let result: Vec<serde_json::Value> = entries
        .into_iter()
        .map(|e| {
            serde_json::json!({
                "name": e.name,
                "path": e.path,
                "is_dir": e.is_dir,
                "size": e.size,
                "modified": e.modified,
            })
        })
        .collect();

    debug!(
        project_id = %project_id, path = %path, count = result.len(),
        "远程目录浏览完成"
    );
    Ok(result)
}

/// 部署 Agent 到远程服务器
///
/// 通过 SFTP 上传 cct-agent 二进制并设置执行权限。
///
/// # 参数
/// - `project_id`: 项目 UUID
///
/// # 返回
/// - `Ok(())`: 部署成功
/// - `Err(CctError)`: 部署失败
#[tauri::command]
pub async fn deploy_agent(project_id: String) -> Result<(), CctError> {
    info!(
        project_id = %project_id,
        "Tauri Command: deploy_agent"
    );

    let ssh_config = load_ssh_config(&project_id).await?;

    let conn = SshConnection::connect(&ssh_config).await?;
    let sftp = SftpClient::from_connection(&conn).await?;

    let install_path = "~/.cct/agent/cct-agent";

    let agent_exists = sftp.exists(install_path).await.unwrap_or(false);
    if agent_exists {
        info!("远程 Agent 已存在，将覆盖安装");
    }

    conn.exec_command("mkdir -p ~/.cct/agent").await.map_err(|e| {
        CctError::AgentDeployFailed(format!("创建安装目录失败: {e}"))
    })?;

    let local_agent = find_local_agent_binary()?;
    sftp.upload_file(&local_agent, install_path).await.map_err(|e| {
        CctError::AgentDeployFailed(format!("上传 Agent 二进制失败: {e}"))
    })?;

    conn.exec_command(&format!("chmod +x {install_path}"))
        .await
        .map_err(|e| {
            CctError::AgentDeployFailed(format!("设置执行权限失败: {e}"))
        })?;

    info!(
        project_id = %project_id,
        install_path = %install_path,
        "Agent 部署完成"
    );
    Ok(())
}

/// 获取远程项目状态
///
/// 连接远程服务器，检查 SSH 可达性和 Agent 运行状态。
///
/// # 参数
/// - `project_id`: 项目 UUID
///
/// # 返回
/// 远程状态 JSON 对象
#[tauri::command]
pub async fn get_remote_status(
    project_id: String,
) -> Result<serde_json::Value, CctError> {
    info!(
        project_id = %project_id,
        "Tauri Command: get_remote_status"
    );

    let ssh_config = match load_ssh_config(&project_id).await {
        Ok(c) => c,
        Err(_) => {
            debug!(project_id = %project_id, "无法加载 SSH 配置，返回断开状态");
            return Ok(serde_json::json!({
                "ssh_state": "Disconnected",
                "agent_state": "NotInstalled",
                "agent_version": null,
                "server_info": null,
                "last_error": null,
            }));
        }
    };

    let conn = match SshConnection::connect(&ssh_config).await {
        Ok(c) => c,
        Err(e) => {
            warn!(error = %e, "SSH 连接失败");
            return Ok(serde_json::json!({
                "ssh_state": "Error",
                "agent_state": "NotInstalled",
                "agent_version": null,
                "server_info": null,
                "last_error": e.to_string(),
            }));
        }
    };

    let agent_bin = "~/.cct/agent/cct-agent";
    let mut agent_version: Option<String> = None;
    let mut agent_state = "NotInstalled";

    match AgentRpcClient::from_connection(&conn, agent_bin).await {
        Ok(mut rpc) => match rpc.get_version().await {
            Ok(version_info) => {
                agent_state = "Running";
                agent_version = version_info
                    .get("version")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());
                info!(version = ?agent_version, "Agent 版本查询成功");
                let _ = rpc.shutdown().await;
            }
            Err(e) => {
                warn!(error = %e, "Agent RPC 失败");
                agent_state = "Error";
            }
        },
        Err(_) => {
            debug!("Agent 未安装或无法启动");
        }
    }

    let result = serde_json::json!({
        "ssh_state": "Connected",
        "agent_state": agent_state,
        "agent_version": agent_version,
        "server_info": null,
        "last_error": null,
    });

    debug!(project_id = %project_id, "远程状态查询完成");
    Ok(result)
}

/// 加载项目的 SSH 配置
///
/// 当前为占位实现，待项目持久化存储集成后替换。
async fn load_ssh_config(project_id: &str) -> Result<SSHConfig, CctError> {
    info!(project_id = %project_id, "load_ssh_config — 加载 SSH 配置");
    warn!(project_id = %project_id, "项目持久化存储待集成，当前返回 ProjectNotFound");
    Err(CctError::ProjectNotFound(project_id.to_string()))
}

/// 查找本地 Agent 二进制文件路径
fn find_local_agent_binary() -> Result<std::path::PathBuf, CctError> {
    info!("find_local_agent_binary — 查找本地 Agent 二进制");

    let candidates = [
        std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|d| d.join("cct-agent"))),
        Some(std::path::PathBuf::from("/tmp/cct-target/release/cct-agent")),
        Some(std::path::PathBuf::from("/tmp/cct-target/debug/cct-agent")),
    ];

    for candidate in &candidates {
        if let Some(path) = candidate {
            if path.exists() {
                debug!(path = %path.display(), "找到 Agent 二进制");
                return Ok(path.clone());
            }
        }
    }

    error!("未找到可用的 Agent 二进制文件");
    Err(CctError::AgentDeployFailed(
        "未找到 cct-agent 二进制文件，请先编译 Agent".to_string(),
    ))
}
