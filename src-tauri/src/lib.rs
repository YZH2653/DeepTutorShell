// 部分工具函数仅供测试/后续扩展
#![allow(dead_code)]

mod config;
mod error;
mod ipc;
mod service;
mod window;

use config::AppConfig;
use service::ServiceState;
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        // 单实例：再次启动时唤起已有窗口，而不是新开一个
        .plugin(tauri_plugin_single_instance::init(|handle, _args, _cwd| {
            if let Some(w) = handle.get_webview_window("main") {
                let _ = w.show();
                let _ = w.unminimize();
                let _ = w.set_focus();
            }
        }))
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let cfg = AppConfig::load();
            app.manage(ServiceState::new(cfg.clone()));
            window::create_main_window(app, &cfg)?;
            service::start_coordinator(app.handle().clone());
            Ok(())
        })
        .on_window_event(|window, event| {
            // 关窗即停服务（含附着模式），阻塞到端口释放再关
            if let tauri::WindowEvent::CloseRequested { .. } = event {
                if window.label() == "main" {
                    service::stop_service(window.app_handle());
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            ipc::get_status,
            ipc::retry_start,
            ipc::open_service_log
        ])
        .build(tauri::generate_context!())
        .expect("error while running tauri application")
        .run(|app, event| {
            // 幂等兜底
            if let tauri::RunEvent::Exit = event {
                service::stop_service(app);
            }
        });
}
