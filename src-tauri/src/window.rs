use crate::config::AppConfig;
use tauri::webview::NewWindowResponse;
use tauri::{Manager, WebviewUrl};

// 创建主窗口（先显示内置加载页，就绪后再导航到 DeepTutor）
pub fn create_main_window(app: &tauri::App, cfg: &AppConfig) -> tauri::Result<tauri::WebviewWindow> {
    let (fe, be) = (cfg.frontend_port, cfg.backend_port);
    tauri::WebviewWindowBuilder::new(app, "main", WebviewUrl::App("index.html".into()))
        .title("DeepTutor")
        .inner_size(1280.0, 800.0)
        .min_inner_size(1024.0, 640.0)
        .on_navigation(move |url| is_allowed_url(url, fe, be))
        .on_new_window(|url, _features| {
            // 外链/弹窗：交给系统浏览器，webview 不新开窗
            let _ = tauri_plugin_opener::open_url(url.as_str(), None::<&str>);
            NewWindowResponse::Deny
        })
        .build()
}

// 导航白名单：内置加载页 + 本机 DeepTutor 端口
pub fn is_allowed_url(url: &url::Url, fe: u16, be: u16) -> bool {
    let host = url.host_str().unwrap_or("");
    let port = url.port_or_known_default();
    // 内置页（生产 tauri://localhost，Windows 下 https://tauri.localhost）
    if url.scheme() == "tauri" && host == "localhost" {
        return true;
    }
    if host == "tauri.localhost" {
        return true;
    }
    // 开发模式 Vite devUrl
    if cfg!(debug_assertions) && host == "localhost" && port == Some(1420) {
        return true;
    }
    // 本机 DeepTutor 端口（仅 http/https）
    if url.scheme() != "http" && url.scheme() != "https" {
        return false;
    }
    (host == "127.0.0.1" || host == "localhost") && (port == Some(fe) || port == Some(be))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn allowed(url: &str) -> bool {
        is_allowed_url(&url::Url::parse(url).unwrap(), 3782, 8001)
    }

    #[test]
    fn whitelist_allows_builtin_and_loopback() {
        assert!(allowed("tauri://localhost/index.html"));
        assert!(allowed("https://tauri.localhost/index.html"));
        assert!(allowed("http://127.0.0.1:3782/"));
        assert!(allowed("http://127.0.0.1:3782/chat"));
        assert!(allowed("http://localhost:3782/"));
        assert!(allowed("http://127.0.0.1:8001/"));
    }

    #[test]
    fn whitelist_rejects_external_and_other_ports() {
        assert!(!allowed("https://evil.com/"));
        assert!(!allowed("http://127.0.0.1:22/"));
        assert!(!allowed("http://127.0.0.1:37820/"));
        assert!(!allowed("http://evil.com:3782/"));
        assert!(!allowed("file:///C:/Windows/System32/calc.exe"));
    }
}

// 就绪后导航到 DeepTutor 前端
pub fn navigate_to_frontend(app: &tauri::AppHandle, fe: u16) {
    if let Some(w) = app.get_webview_window("main") {
        if let Ok(u) = url::Url::parse(&format!("http://127.0.0.1:{fe}/")) {
            let _ = w.navigate(u);
        }
    }
}
