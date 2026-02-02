// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod api;
mod models;
mod engine;

use engine::analyzer;
use models::{article::Article, market::Market};
use tauri::Emitter;

#[tauri::command]
fn run_full_analysis(app: tauri::AppHandle, budget: Option<f64>) -> Result<(), String> {
    let app_handle = app.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let result = tauri::async_runtime::block_on(analyzer::run_full_analysis(app_handle.clone(), budget, None));
        match result {
            Ok(recommendations) => {
                let _ = app_handle.emit("analysis_result", recommendations);
            }
            Err(err) => {
                let _ = app_handle.emit("analysis_error", err);
            }
        }
    });
    Ok(())
}

async fn fetch_top_stories() -> Result<Vec<Article>, String> {
    api::hackernews::fetch_top_stories()
        .await
        .map_err(|e| e.to_string())
}

async fn fetch_active_markets() -> Result<Vec<Market>, String> {
    api::polymarket::fetch_active_markets().await
}

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            run_full_analysis
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
