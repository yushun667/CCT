mod ai;
mod commands;
mod logging;
mod services;

use tracing::info;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let _guard = logging::init();

    info!("CCT 桌面端启动");

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .invoke_handler(tauri::generate_handler![
            commands::get_app_config,
            commands::save_app_config,
            commands::get_app_version,
            commands::project::create_local_project,
            commands::project::create_remote_project,
            commands::project::list_projects,
            commands::project::get_project,
            commands::project::update_project,
            commands::project::delete_project,
            commands::parse::start_full_parse,
            commands::parse::cancel_parse,
            commands::parse::get_parse_status,
            commands::parse::get_parse_statistics,
            commands::parse::start_incremental_parse,
            commands::parse::get_parse_errors,
            commands::remote::test_ssh_connection,
            commands::remote::browse_remote_dir,
            commands::remote::deploy_agent,
            commands::remote::get_remote_status,
            commands::remote::browse_remote_dir_temp,
            commands::remote::deploy_agent_temp,
            commands::query::search_symbols,
            commands::query::query_callers,
            commands::query::query_callees,
            commands::query::query_references,
            commands::query::query_call_path,
            commands::editor::read_file_content,
            commands::editor::get_file_symbols,
            commands::graph::get_call_graph,
            commands::graph::get_file_dependency_graph,
            commands::ai::ai_chat,
            commands::ai::ai_stop,
            commands::ai::list_conversations,
            commands::ai::get_conversation,
            commands::ai::delete_conversation,
            commands::ai::get_ai_config,
            commands::ai::update_ai_config,
            commands::ai::list_ai_skills,
            commands::analysis::list_syscalls,
            commands::analysis::get_syscall_path,
            commands::analysis::list_ioctl_commands,
            commands::analysis::list_ipc_services,
            commands::analysis::get_ipc_call_path,
            commands::analysis::load_custom_rules,
            commands::analysis::apply_custom_rules,
        ])
        .run(tauri::generate_context!())
        .expect("CCT 桌面端启动失败");
}
