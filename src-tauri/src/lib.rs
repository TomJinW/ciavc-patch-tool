use tauri_plugin_cli::CliExt; 
use tauri::{Emitter};

// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_cli::init()) // 1. 註冊插件
        .setup(|app| {
            let handle = app.handle().clone();
            match app.cli().matches() {
                Ok(matches) => {
                    // 建議使用 spawn，避免阻塞主線程，並稍微等前端準備好
                    tauri::async_runtime::spawn(async move {
                        // 給前端 500ms 啟動並監聽事件 (這是一個簡單的保險)
                        std::thread::sleep(std::time::Duration::from_millis(500));
                        handle.emit("cli-event", matches.args).unwrap();
                    });
                }
                Err(e) => eprintln!("CLI Error: {}", e),
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![greet])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
