//! Tauri shell around `envenb-core`.
//!
//! The command surface below is the *entire* IPC contract with the React side.
//! It mirrors the Local Agent's request set: listing returns metadata only, and
//! no command returns a secret value. Writing a secret is the one place plaintext
//! crosses from the webview to Rust, where it is wrapped in `SecretValue` at once.

mod commands;

use envenb_core::EnvEnb;

pub struct AppState {
    pub core: EnvEnb,
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    init_tracing();

    let core = tauri::async_runtime::block_on(EnvEnb::open_default())
        .unwrap_or_else(|err| panic!("failed to open EnvEnb data directory: {err}"));

    tauri::Builder::default()
        .manage(AppState { core })
        .setup(|app| {
            // Opt-in devtools for debugging the webview: ENVENB_DEVTOOLS=1 pnpm dev
            #[cfg(debug_assertions)]
            if envenb_core::env_compat::is_set("DEVTOOLS") {
                use tauri::Manager;
                if let Some(window) = app.get_webview_window("main") {
                    window.open_devtools();
                }
            }
            let _ = app;
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::status,
            commands::get_settings,
            commands::set_language,
            commands::set_theme,
            commands::list_projects,
            commands::create_project,
            commands::delete_project,
            commands::list_environments,
            commands::create_environment,
            commands::delete_environment,
            commands::list_variables,
            commands::set_public_variable,
            commands::set_secret_variable,
            commands::change_variable_kind,
            commands::delete_variable,
            commands::render_env_example,
            commands::preview_dotenv,
            commands::import_variables,
            commands::list_connections,
            commands::create_connection,
            commands::delete_connection,
            commands::list_ai_clients,
            commands::register_ai_client,
            commands::delete_ai_client,
            commands::list_permissions,
            commands::set_permission,
            commands::delete_permission,
            commands::effective_decision,
            commands::list_approvals,
            commands::resolve_approval,
            commands::list_audit,
            commands::list_credentials,
            commands::credential_field_specs,
            commands::create_credential,
            commands::update_credential_fields,
            commands::delete_credential,
            commands::copy_credential_field,
            commands::ensure_gitignore,
            commands::delete_dotenv_file,
        ])
        .run(tauri::generate_context!())
        .expect("error while running EnvEnb");
}

fn init_tracing() {
    use tracing_subscriber::EnvFilter;
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("envenb=info"));
    let _ = tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(false)
        .try_init();
}
