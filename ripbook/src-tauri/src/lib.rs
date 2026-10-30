mod commands;
mod disc;
mod freac;
mod freac_install;
mod manifest;
mod models;
#[cfg(windows)]
mod powershell;
mod settings;

use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.unminimize();
                let _ = window.set_focus();
            }
        }))
        .plugin(tauri_plugin_dialog::init())
        .manage(commands::AppState::default())
        .invoke_handler(tauri::generate_handler![
            commands::get_app_info,
            commands::list_books,
            commands::create_book,
            commands::load_book,
            commands::set_last_disc,
            commands::list_drives,
            commands::lookup_disc_metadata,
            commands::probe_freaccmd,
            commands::install_freac,
            commands::rip_disc,
            commands::cancel_rip,
            commands::eject_drive,
            commands::open_folder,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
