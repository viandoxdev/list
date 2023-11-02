#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

use tauri::Manager;

pub fn main() {
    app::AppBuilder::new()
        .setup(|app| {
            app.get_window("main").unwrap().open_devtools();
            Ok(())
        })
        .run();
}
