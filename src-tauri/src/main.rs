// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod api;
mod models;

use api::hackernews::fetch_top_stories;

fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![fetch_top_stories])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
