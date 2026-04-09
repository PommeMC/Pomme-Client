#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod auth;
mod commands;
mod downloader;
mod installations;
mod ping;
mod settings;
mod storage;

use std::collections::VecDeque;
use tauri::Manager;
use tokio::sync::Mutex;

const TYPED_ERROR_IMPL: &str = r#"export type Result<T, E> =
  | { ok: true;  value: T }
  | { ok: false; error: E };

export const Result = {
  ok<T>(value: T): Result<T, never> {
    return { ok: true, value };
  },
  err<E>(error: E): Result<never, E> {
    return { ok: false, error };
  },
} as const;


export async function typedError<T, E>(promise: Promise<T>): Promise<Result<T, E>> {
  try {
    return Result.ok(await promise);
  } catch (e) {
    if (e instanceof Error) throw e;
    return Result.err(e as E);
  }
}"#;

#[derive(Default)]
pub struct AppState {
    pub client_logs: Mutex<VecDeque<String>>,
    pub installations_lock: Mutex<()>,
}

fn main() {
    #[cfg(target_os = "linux")]
    if std::env::var("WEBKIT_DISABLE_COMPOSITING_MODE").is_err() {
        unsafe { std::env::set_var("WEBKIT_DISABLE_COMPOSITING_MODE", "0") };
    }

    let builder = tauri_specta::Builder::new()
        .commands(tauri_specta::collect_commands![
            commands::get_all_accounts,
            commands::add_account,
            commands::remove_account,
            commands::ensure_assets,
            commands::get_versions,
            commands::refresh_account,
            commands::get_skin_url,
            commands::get_patch_notes,
            commands::get_patch_content,
            commands::launch_game,
            commands::get_client_logs,
            commands::load_launcher_settings,
            commands::set_launcher_language,
            commands::set_keep_launcher_open,
            commands::set_launch_with_console,
            commands::ping_server,
            commands::load_servers,
            commands::save_servers,
            commands::load_installations,
            commands::create_installation,
            commands::delete_installation,
            commands::duplicate_installation,
            commands::edit_installation,
            commands::get_downloaded_versions,
        ])
        .typed_error_impl(TYPED_ERROR_IMPL);

    #[cfg(debug_assertions)]
    if let Err(e) = builder.export(
        specta_typescript::Typescript::default()
            .layout(specta_typescript::Layout::Files)
            .header("// @ts-nocheck\n/* eslint-disable */"),
        "../src/bindings/",
    ) {
        panic!("{e}");
    };

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            storage::ensure_dirs();
            app.manage(AppState::default());
            Ok(())
        })
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(builder.invoke_handler())
        .run(tauri::generate_context!())
        .expect("failed to run Pomme launcher");
}
