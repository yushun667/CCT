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
            commands::project::list_projects,
            commands::project::get_project,
            commands::project::update_project,
            commands::project::delete_project,
            commands::parse::start_full_parse,
            commands::parse::cancel_parse,
            commands::parse::get_parse_status,
            commands::parse::get_parse_statistics,
        ])
        .run(tauri::generate_context!())
        .expect("CCT 桌面端启动失败");
}
