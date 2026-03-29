// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

pub mod bindings;
pub mod checkpoint;
pub mod claude_binary;
pub mod governed_claude;
pub mod commands;
pub mod process;
pub mod sidecars;
pub mod types;
pub mod utils;
pub mod web_server;

use checkpoint::state::CheckpointState;
use commands::agents::{
    cleanup_finished_processes, create_agent, delete_agent, execute_agent, export_agent,
    export_agent_to_file, fetch_github_agent_content, fetch_github_agents, get_agent,
    get_agent_run, get_agent_run_with_real_time_metrics, get_claude_binary_path,
    get_live_session_output, get_session_output, get_session_status, import_agent,
    import_agent_from_file, import_agent_from_github, init_database, kill_agent_session,
    list_agent_runs, list_agent_runs_with_metrics, list_agents, list_claude_installations,
    list_running_sessions, load_agent_session_history, set_claude_binary_path,
    stream_session_output, update_agent, AgentDb,
};
use commands::claude::{
    cancel_claude_execution, check_auto_checkpoint, check_claude_version, cleanup_old_checkpoints,
    clear_checkpoint_manager, continue_claude_code, create_checkpoint, create_project,
    execute_claude_code, find_claude_md_files, fork_from_checkpoint, get_checkpoint_diff,
    get_checkpoint_settings, get_checkpoint_state_stats, get_claude_session_output,
    get_claude_settings, get_home_directory, get_hooks_config, get_project_sessions,
    get_recently_modified_files, get_session_timeline, get_system_prompt, list_checkpoints,
    list_directory_contents, list_projects, list_running_claude_sessions, load_session_history,
    open_new_session, read_claude_md_file, restore_checkpoint, resume_claude_code,
    save_claude_md_file, save_claude_settings, save_system_prompt, search_files,
    track_checkpoint_message, track_session_messages, update_checkpoint_settings,
    update_hooks_config, validate_hook_command, ClaudeProcessState,
};
use commands::mcp::{
    mcp_add, mcp_add_from_claude_desktop, mcp_add_json, mcp_get, mcp_get_server_status, mcp_list,
    mcp_read_project_config, mcp_remove, mcp_reset_project_choices, mcp_save_project_config,
    mcp_serve, mcp_test_connection,
};
use commands::proxy::{apply_proxy_settings, get_proxy_settings, save_proxy_settings};
use commands::storage::{
    storage_delete_row, storage_execute_sql, storage_insert_row, storage_list_tables,
    storage_read_table, storage_reset_database, storage_update_row,
};
use commands::usage::{
    get_session_stats, get_usage_by_date_range, get_usage_details, get_usage_stats,
};
use process::ProcessRegistryState;
use sidecars::SidecarState;
use std::sync::Mutex;
use tauri::Manager;

#[cfg(target_os = "macos")]
use window_vibrancy::{apply_vibrancy, NSVisualEffectMaterial};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Export TypeScript bindings on every debug start so the frontend stays in sync.
    #[cfg(debug_assertions)]
    bindings::export_ts_bindings();

    // Build the plugin chain. The MCP bridge is conditionally added when the
    // `mcp-dev` feature is enabled — it exposes the running app to AI assistants
    // (Claude Code, Cursor, etc.) via a WebSocket on port 9223.
    // Linux is excluded due to an upstream glib version conflict in the plugin.
    let builder = tauri::Builder::default()
        // Single-instance must be first to gate startup before any state is set up
        .plugin(tauri_plugin_single_instance::init(|app, _argv, _cwd| {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.show();
                let _ = window.set_focus();
            }
        }))
        .plugin(
            tauri_plugin_log::Builder::new()
                .targets([
                    tauri_plugin_log::Target::new(tauri_plugin_log::TargetKind::Stdout),
                    tauri_plugin_log::Target::new(tauri_plugin_log::TargetKind::LogDir {
                        file_name: Some("opc".into()),
                    }),
                    tauri_plugin_log::Target::new(tauri_plugin_log::TargetKind::Webview),
                ])
                .level(if cfg!(debug_assertions) {
                    log::LevelFilter::Debug
                } else {
                    log::LevelFilter::Info
                })
                .build(),
        )
        .plugin(tauri_plugin_window_state::Builder::default().build())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            None,
        ))
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_persisted_scope::init())
        .plugin(tauri_plugin_deep_link::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_os::init());

    #[cfg(all(feature = "mcp-dev", any(target_os = "macos", target_os = "windows")))]
    let builder = builder.plugin(tauri_plugin_mcp_bridge::init());

    builder.setup(|app| {
            // Initialize agents database
            let conn = init_database(&app.handle()).expect("Failed to initialize agents database");

            // Load and apply proxy settings from the database at startup
            {
                let db = AgentDb(Mutex::new(conn));
                let proxy_settings = match db.0.lock() {
                    Ok(conn) => {
                        let mut settings = commands::proxy::ProxySettings::default();
                        let keys = vec![
                            ("proxy_enabled", "enabled"),
                            ("proxy_http", "http_proxy"),
                            ("proxy_https", "https_proxy"),
                            ("proxy_no", "no_proxy"),
                            ("proxy_all", "all_proxy"),
                        ];
                        for (db_key, field) in keys {
                            if let Ok(value) = conn.query_row(
                                "SELECT value FROM app_settings WHERE key = ?1",
                                rusqlite::params![db_key],
                                |row| row.get::<_, String>(0),
                            ) {
                                match field {
                                    "enabled" => settings.enabled = value == "true",
                                    "http_proxy" => {
                                        settings.http_proxy = Some(value).filter(|s| !s.is_empty())
                                    }
                                    "https_proxy" => {
                                        settings.https_proxy =
                                            Some(value).filter(|s| !s.is_empty())
                                    }
                                    "no_proxy" => {
                                        settings.no_proxy = Some(value).filter(|s| !s.is_empty())
                                    }
                                    "all_proxy" => {
                                        settings.all_proxy = Some(value).filter(|s| !s.is_empty())
                                    }
                                    _ => {}
                                }
                            }
                        }
                        log::info!("Loaded proxy settings: enabled={}", settings.enabled);
                        settings
                    }
                    Err(e) => {
                        log::warn!("Failed to lock database for proxy settings: {}", e);
                        commands::proxy::ProxySettings::default()
                    }
                };
                apply_proxy_settings(&proxy_settings);
            }

            // Re-open the connection for managed state
            let conn = init_database(&app.handle()).expect("Failed to initialize agents database");
            app.manage(AgentDb(Mutex::new(conn)));

            // Initialize checkpoint state
            let checkpoint_state = CheckpointState::new();
            if let Ok(claude_dir) = dirs::home_dir()
                .ok_or_else(|| "Could not find home directory")
                .and_then(|home| {
                    let claude_path = home.join(".claude");
                    claude_path
                        .canonicalize()
                        .map_err(|_| "Could not find ~/.claude directory")
                })
            {
                let state_clone = checkpoint_state.clone();
                tauri::async_runtime::spawn(async move {
                    state_clone.set_claude_dir(claude_dir).await;
                });
            }
            app.manage(checkpoint_state);

            app.manage(commands::titor::TitorState::new());

            // Initialize process registry and Claude process state
            app.manage(ProcessRegistryState::default());
            app.manage(ClaudeProcessState::default());

            // Initialize sidecar state (ports populated when sidecars announce them)
            app.manage(SidecarState::default());
            sidecars::spawn_axiomregent(app.handle());

            // Initialize zoom state (Tauri 2 has no zoom getter; we track it here)
            app.manage(commands::window_ctrl::ZoomState::default());

            // Initialize quick pane (hidden, shown via global shortcut)
            if let Err(e) = commands::quick_pane::init_quick_pane(app.handle()) {
                log::error!("Failed to create quick pane: {e}");
                // Non-fatal: app can still run without quick pane
            }

            // Register global shortcut for quick pane
            #[cfg(desktop)]
            {
                use tauri_plugin_global_shortcut::Builder as ShortcutBuilder;
                if let Err(e) = app.handle().plugin(ShortcutBuilder::new().build()) {
                    log::error!("Failed to initialize global shortcut plugin: {e}");
                }
                if let Err(e) = commands::quick_pane::register_quick_pane_shortcut(
                    app.handle(),
                    commands::quick_pane::DEFAULT_QUICK_PANE_SHORTCUT,
                ) {
                    log::warn!("Failed to register quick pane shortcut: {e}");
                }
            }

            // Apply window vibrancy with rounded corners on macOS
            #[cfg(target_os = "macos")]
            {
                let window = app.get_webview_window("main").unwrap();
                let materials = [
                    NSVisualEffectMaterial::UnderWindowBackground,
                    NSVisualEffectMaterial::WindowBackground,
                    NSVisualEffectMaterial::Popover,
                    NSVisualEffectMaterial::Menu,
                    NSVisualEffectMaterial::Sidebar,
                ];
                let mut applied = false;
                for material in materials.iter() {
                    if apply_vibrancy(&window, *material, None, Some(12.0)).is_ok() {
                        applied = true;
                        break;
                    }
                }
                if !applied {
                    apply_vibrancy(
                        &window,
                        NSVisualEffectMaterial::WindowBackground,
                        None,
                        None,
                    )
                    .expect("Failed to apply any window vibrancy");
                }
            }

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            // Claude & Project Management
            list_projects,
            create_project,
            get_project_sessions,
            get_home_directory,
            get_claude_settings,
            open_new_session,
            get_system_prompt,
            check_claude_version,
            save_system_prompt,
            save_claude_settings,
            find_claude_md_files,
            read_claude_md_file,
            save_claude_md_file,
            load_session_history,
            execute_claude_code,
            continue_claude_code,
            resume_claude_code,
            cancel_claude_execution,
            list_running_claude_sessions,
            get_claude_session_output,
            list_directory_contents,
            search_files,
            get_recently_modified_files,
            get_hooks_config,
            update_hooks_config,
            validate_hook_command,
            // Checkpoint Management
            create_checkpoint,
            restore_checkpoint,
            list_checkpoints,
            fork_from_checkpoint,
            get_session_timeline,
            update_checkpoint_settings,
            get_checkpoint_diff,
            track_checkpoint_message,
            track_session_messages,
            check_auto_checkpoint,
            cleanup_old_checkpoints,
            get_checkpoint_settings,
            clear_checkpoint_manager,
            get_checkpoint_state_stats,
            // Agent Management
            list_agents,
            create_agent,
            update_agent,
            delete_agent,
            get_agent,
            execute_agent,
            list_agent_runs,
            get_agent_run,
            list_agent_runs_with_metrics,
            get_agent_run_with_real_time_metrics,
            list_running_sessions,
            kill_agent_session,
            get_session_status,
            cleanup_finished_processes,
            get_session_output,
            get_live_session_output,
            stream_session_output,
            load_agent_session_history,
            get_claude_binary_path,
            set_claude_binary_path,
            list_claude_installations,
            export_agent,
            export_agent_to_file,
            import_agent,
            import_agent_from_file,
            fetch_github_agents,
            fetch_github_agent_content,
            import_agent_from_github,
            // Usage & Analytics
            get_usage_stats,
            get_usage_by_date_range,
            get_usage_details,
            get_session_stats,
            // MCP (Model Context Protocol)
            mcp_add,
            mcp_list,
            mcp_get,
            mcp_remove,
            mcp_add_json,
            mcp_add_from_claude_desktop,
            mcp_serve,
            mcp_test_connection,
            mcp_reset_project_choices,
            mcp_get_server_status,
            mcp_read_project_config,
            mcp_save_project_config,
            // Storage Management
            storage_list_tables,
            storage_read_table,
            storage_update_row,
            storage_delete_row,
            storage_insert_row,
            storage_execute_sql,
            storage_reset_database,
            // Slash Commands
            commands::slash_commands::slash_commands_list,
            commands::slash_commands::slash_command_get,
            commands::slash_commands::slash_command_save,
            commands::slash_commands::slash_command_delete,
            // Proxy Settings
            get_proxy_settings,
            save_proxy_settings,
            // Titor Checkpointing
            commands::titor::titor_init,
            commands::titor::titor_checkpoint,
            commands::titor::titor_list,
            commands::titor::titor_restore,
            commands::titor::titor_diff,
            commands::titor::titor_verify,
            // Xray & Featuregraph Analysis
            commands::analysis::xray_scan_project,
            commands::analysis::featuregraph_overview,
            commands::analysis::featuregraph_impact,
            commands::analysis::get_preflight_safety_tier_reference,
            commands::analysis::get_tool_tier_assignments,
            // Blockoli & Stackwalk Search
            commands::search::blockoli_index_project,
            commands::search::blockoli_search,
            commands::search::search_codebase,
            // MCP proxy commands
            commands::mcp::mcp_list_tools,
            commands::mcp::mcp_call_tool,
            commands::mcp::mcp_read_resource,
            // Recovery system
            commands::recovery::save_emergency_data,
            commands::recovery::load_emergency_data,
            commands::recovery::cleanup_old_recovery_files,
            // Updater with SHA-256 verification
            commands::updater::check_for_update,
            commands::updater::download_and_install_update,
            // Native git operations
            commands::git::git_diff,
            commands::git::git_status,
            commands::git::git_ahead_behind,
            commands::git::git_current_branch,
            // Quick pane window management
            commands::quick_pane::show_quick_pane,
            commands::quick_pane::dismiss_quick_pane,
            commands::quick_pane::toggle_quick_pane,
            commands::quick_pane::get_default_quick_pane_shortcut,
            commands::quick_pane::update_quick_pane_shortcut,
            commands::quick_pane::set_quick_pane_enabled,
            // Window control (zoom, progress bar, bounds, WSL)
            commands::window_ctrl::set_zoom,
            commands::window_ctrl::get_zoom,
            commands::window_ctrl::broadcast_zoom,
            commands::window_ctrl::set_progress_bar,
            commands::window_ctrl::get_window_bounds,
            commands::window_ctrl::set_window_bounds,
            commands::window_ctrl::is_wsl,
            // WSL distro listing and execution
            commands::wsl::wsl_is_available,
            commands::wsl::wsl_list_distros,
            commands::wsl::wsl_execute,
            // Sandbox status
            commands::sandbox::sandbox_status,
            // Sidecar port discovery
            sidecars::get_sidecar_ports,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
