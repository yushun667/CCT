mod ai;
mod commands;
mod logging;
mod services;

use tracing::{info, warn};
use tauri::Emitter;
use tauri::menu::{MenuBuilder, SubmenuBuilder, MenuItemBuilder};

fn recover_stale_parse_status() {
    use cct_core::models::project::ParseStatus;
    let service = services::project_service::ProjectService::from_default();
    if let Ok(projects) = service.list() {
        for p in &projects {
            if p.parse_status == ParseStatus::InProgress {
                warn!(
                    id = %p.id,
                    name = %p.name,
                    "检测到遗留的 InProgress 状态，重置为 NotStarted"
                );
                let _ = service.update_parse_status(&p.id, ParseStatus::NotStarted);
            }
        }
    }
}

fn build_native_menu(app: &tauri::App) -> Result<tauri::menu::Menu<tauri::Wry>, tauri::Error> {
    let open_dir = MenuItemBuilder::with_id("open_directory", "打开目录...")
        .accelerator("CmdOrCtrl+O")
        .build(app)?;
    let parse = MenuItemBuilder::with_id("start_parse", "开始解析")
        .accelerator("CmdOrCtrl+Shift+P")
        .build(app)?;
    let project_settings = MenuItemBuilder::with_id("project_settings", "项目设置...")
        .accelerator("CmdOrCtrl+,")
        .build(app)?;

    let app_settings = MenuItemBuilder::with_id("app_settings", "应用设置...")
        .build(app)?;

    let file_menu = SubmenuBuilder::new(app, "文件")
        .item(&open_dir)
        .separator()
        .item(&parse)
        .item(&project_settings)
        .separator()
        .item(&app_settings)
        .separator()
        .close_window()
        .quit()
        .build()?;

    let edit_menu = SubmenuBuilder::new(app, "编辑")
        .undo()
        .redo()
        .separator()
        .cut()
        .copy()
        .paste()
        .select_all()
        .build()?;

    let toggle_sidebar = MenuItemBuilder::with_id("toggle_sidebar", "切换侧边栏")
        .accelerator("CmdOrCtrl+B")
        .build(app)?;
    let toggle_terminal = MenuItemBuilder::with_id("toggle_terminal", "切换终端")
        .accelerator("Ctrl+`")
        .build(app)?;
    let toggle_ai = MenuItemBuilder::with_id("toggle_ai", "切换 AI 面板")
        .accelerator("CmdOrCtrl+Shift+A")
        .build(app)?;

    let view_menu = SubmenuBuilder::new(app, "查看")
        .item(&toggle_sidebar)
        .item(&toggle_terminal)
        .item(&toggle_ai)
        .separator()
        .fullscreen()
        .build()?;

    let menu = MenuBuilder::new(app)
        .item(&file_menu)
        .item(&edit_menu)
        .item(&view_menu)
        .build()?;

    Ok(menu)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let _guard = logging::init();

    info!("CCT 桌面端启动");

    recover_stale_parse_status();

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .setup(|app| {
            let menu = build_native_menu(app)?;
            app.set_menu(menu)?;

            let app_handle = app.handle().clone();
            app.on_menu_event(move |_app, event| {
                let id = event.id().0.as_str();
                let _ = app_handle.emit(id, ());
            });

            Ok(())
        })
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
            commands::remote::test_ssh_connection_with_config,
            commands::remote::browse_remote_dir,
            commands::remote::deploy_agent,
            commands::remote::get_remote_status,
            commands::remote::browse_remote_dir_temp,
            commands::remote::deploy_agent_temp,
            commands::query::search_symbols,
            commands::query::get_symbols_by_ids,
            commands::query::query_callers,
            commands::query::query_callees,
            commands::query::query_references,
            commands::query::query_call_path,
            commands::editor::list_directory,
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
