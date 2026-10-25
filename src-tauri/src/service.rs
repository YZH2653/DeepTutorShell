use crate::config::{exe_dir, AppConfig};
use crate::window;
use serde::Serialize;
use std::io::{Read, Write};
use std::os::windows::process::CommandExt;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Mutex;
use tauri::{Emitter, Manager};

const CREATE_NO_WINDOW: u32 = 0x0800_0000;
const STARTUP_TIMEOUT_SECS: u64 = 60;
const POLL_INTERVAL_MS: u64 = 500;

// 拉起的 DeepTutor 服务句柄
pub struct ServiceHandle {
    pub child: std::process::Child,
    pub log_path: PathBuf,
}

// 加载页状态载荷（service:status 事件 / get_status 快照）
#[derive(Clone, Serialize)]
pub struct StatusPayload {
    pub state: String,
    pub mode: String,
    pub message: String,
    pub log_path: String,
}

// 服务运行状态（全局托管）
pub struct ServiceState {
    pub cfg: AppConfig,
    child: Mutex<Option<std::process::Child>>,
    log_path: Mutex<Option<PathBuf>>,
    generation: AtomicU64,
    stop_guard: AtomicBool,
    last_status: Mutex<StatusPayload>,
}

impl ServiceState {
    pub fn new(cfg: AppConfig) -> Self {
        Self {
            cfg,
            child: Mutex::new(None),
            log_path: Mutex::new(None),
            generation: AtomicU64::new(0),
            stop_guard: AtomicBool::new(false),
            last_status: Mutex::new(StatusPayload {
                state: "starting".into(),
                mode: "spawn".into(),
                message: String::new(),
                log_path: String::new(),
            }),
        }
    }

    // 自拉进程 PID
    pub fn child_pid(&self) -> Option<u32> {
        self.child
            .lock()
            .ok()
            .and_then(|c| c.as_ref().map(|c| c.id()))
    }

    pub fn log_path(&self) -> Option<PathBuf> {
        self.log_path.lock().ok().and_then(|p| p.clone())
    }

    pub fn last_status(&self) -> StatusPayload {
        self.last_status.lock().unwrap().clone()
    }
}

// 后台拉起 deeptutor start（无 --detach，保留进程树可整体杀掉）
pub fn spawn_service(cfg: &AppConfig) -> std::io::Result<ServiceHandle> {
    let log_path = exe_dir().join("logs").join("deeptutor-service.log");
    if let Some(dir) = log_path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let out = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_path)?;
    let err = out.try_clone()?;
    let child = std::process::Command::new(&cfg.deeptutor_exe)
        .args(["start", "--no-browser", "--home", &cfg.workspace])
        .current_dir(&cfg.workspace)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::from(out))
        .stderr(std::process::Stdio::from(err))
        .creation_flags(CREATE_NO_WINDOW)
        .spawn()?;
    Ok(ServiceHandle { child, log_path })
}

// 端口是否在监听（裸 TCP，不走系统代理）
pub fn port_listening(port: u16) -> bool {
    std::net::TcpStream::connect(("127.0.0.1", port)).is_ok()
}

// 解析 HTTP 状态行
pub fn parse_http_status(head: &str) -> Option<u16> {
    let mut it = head.split_whitespace();
    if !it.next()?.starts_with("HTTP/") {
        return None;
    }
    it.next()?.parse().ok()
}

// 探测前端就绪（HTTP 2xx/3xx 视为就绪）
pub fn probe_http(port: u16) -> bool {
    let Ok(mut s) = std::net::TcpStream::connect(("127.0.0.1", port)) else {
        return false;
    };
    let _ = s.set_read_timeout(Some(std::time::Duration::from_secs(3)));
    let req = format!("GET / HTTP/1.0\r\nHost: 127.0.0.1:{port}\r\nConnection: close\r\n\r\n");
    if s.write_all(req.as_bytes()).is_err() {
        return false;
    }
    let mut buf = [0u8; 128];
    let n = s.read(&mut buf).unwrap_or(0);
    parse_http_status(&String::from_utf8_lossy(&buf[..n]))
        .is_some_and(|c| (200..400).contains(&c))
}

// 启动协调器（可重复调用；旧轮询靠 generation 自动作废）
pub fn start_coordinator(app: tauri::AppHandle) {
    let gen = app
        .state::<ServiceState>()
        .generation
        .fetch_add(1, Ordering::SeqCst)
        + 1;
    tauri::async_runtime::spawn(async move {
        run_coordinator(app, gen).await;
    });
}

// 探测 → 按需拉起 → 轮询就绪 → 导航到 DeepTutor
async fn run_coordinator(app: tauri::AppHandle, gen: u64) {
    let cfg = app.state::<ServiceState>().cfg.clone();
    // 已就绪=附着；端口在听=他人启动中；都不通=自行拉起
    let mode = if probe_http(cfg.frontend_port) {
        "attach"
    } else if port_listening(cfg.frontend_port) || port_listening(cfg.backend_port) {
        "attach_pending"
    } else {
        "spawn"
    };
    let mut log_path = String::new();
    if mode == "spawn" {
        match spawn_service(&cfg) {
            Ok(h) => {
                log_path = h.log_path.to_string_lossy().to_string();
                let state = app.state::<ServiceState>();
                *state.child.lock().unwrap() = Some(h.child);
                *state.log_path.lock().unwrap() = Some(h.log_path);
            }
            Err(e) => {
                emit_status(&app, "error", mode, &format!("拉起 DeepTutor 失败：{e}"), "");
                return;
            }
        }
    } else if let Some(p) = app.state::<ServiceState>().log_path() {
        log_path = p.to_string_lossy().to_string();
    }
    emit_status(&app, "starting", mode, "", &log_path);
    let deadline =
        std::time::Instant::now() + std::time::Duration::from_secs(STARTUP_TIMEOUT_SECS);
    while std::time::Instant::now() < deadline {
        // 被 retry 取代则退出
        if app.state::<ServiceState>().generation.load(Ordering::SeqCst) != gen {
            return;
        }
        if probe_http(cfg.frontend_port) {
            emit_status(&app, "ready", mode, "", &log_path);
            window::navigate_to_frontend(&app, cfg.frontend_port);
            return;
        }
        tokio::time::sleep(std::time::Duration::from_millis(POLL_INTERVAL_MS)).await;
    }
    emit_status(&app, "timeout", mode, "服务在 60 秒内未就绪", &log_path);
}

// 发布状态（更新快照 + 通知加载页）
fn emit_status(app: &tauri::AppHandle, state: &str, mode: &str, message: &str, log_path: &str) {
    let payload = StatusPayload {
        state: state.to_string(),
        mode: mode.to_string(),
        message: message.to_string(),
        log_path: log_path.to_string(),
    };
    *app.state::<ServiceState>().last_status.lock().unwrap() = payload.clone();
    let _ = app.emit("service:status", payload);
}

// 关窗即停服务（幂等；阻塞到进程死透，保证端口释放）
pub fn stop_service(app: &tauri::AppHandle) {
    let state = app.state::<ServiceState>();
    if state.stop_guard.swap(true, Ordering::SeqCst) {
        return;
    }
    // 自拉的进程树
    if let Some(pid) = state.child_pid() {
        taskkill_tree(pid);
    }
    // 端口扫尾：覆盖附着模式与孤儿进程
    let ports = [state.cfg.frontend_port, state.cfg.backend_port];
    for pid in pids_listening_on(&ports) {
        taskkill_tree(pid);
    }
}

// taskkill 参数（纯函数，便于测试）
pub fn taskkill_args(pid: u32) -> Vec<String> {
    vec![
        "/PID".to_string(),
        pid.to_string(),
        "/T".to_string(),
        "/F".to_string(),
    ]
}

// 杀掉整棵进程树（.status() 阻塞到结束）
fn taskkill_tree(pid: u32) {
    let _ = std::process::Command::new("taskkill")
        .args(taskkill_args(pid))
        .creation_flags(CREATE_NO_WINDOW)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status();
}

// 查找监听指定端口的 PID
fn pids_listening_on(ports: &[u16]) -> Vec<u32> {
    let Ok(out) = std::process::Command::new("netstat")
        .args(["-ano", "-p", "tcp"])
        .creation_flags(CREATE_NO_WINDOW)
        .output()
    else {
        return Vec::new();
    };
    parse_netstat_pids(&String::from_utf8_lossy(&out.stdout), ports)
}

// 解析 netstat 输出，取监听指定端口的 PID（纯函数）
pub fn parse_netstat_pids(out: &str, ports: &[u16]) -> Vec<u32> {
    let mut pids = std::collections::BTreeSet::new();
    for line in out.lines() {
        let mut c = line.split_whitespace();
        if !c.next().is_some_and(|p| p.eq_ignore_ascii_case("TCP")) {
            continue;
        }
        let Some(local) = c.next() else { continue };
        let Some(_foreign) = c.next() else { continue };
        if !c.next().is_some_and(|p| p.eq_ignore_ascii_case("LISTENING")) {
            continue;
        }
        let Some(pid) = c.next().and_then(|p| p.parse::<u32>().ok()) else {
            continue;
        };
        let Some(port) = local.rsplit(':').next().and_then(|p| p.parse::<u16>().ok()) else {
            continue;
        };
        if pid != 0 && ports.contains(&port) {
            pids.insert(pid);
        }
    }
    pids.into_iter().collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_http_status_cases() {
        assert_eq!(parse_http_status("HTTP/1.1 200 OK"), Some(200));
        assert_eq!(
            parse_http_status("HTTP/1.0 307 Temporary Redirect"),
            Some(307)
        );
        assert_eq!(parse_http_status("garbage"), None);
        assert_eq!(parse_http_status(""), None);
        assert_eq!(parse_http_status("HTTP/x"), None);
    }

    // netstat 夹具：IPv4/IPv6 LISTENING、ESTABLISHED、UDP、PID 0
    #[test]
    fn parse_netstat_pids_cases() {
        let out = "\
Active Connections\r\n\
\r\n\
  Proto  Local Address          Foreign Address        State           PID\r\n\
  TCP    0.0.0.0:3782           0.0.0.0:0              LISTENING       27292\r\n\
  TCP    127.0.0.1:52344        127.0.0.1:3782         ESTABLISHED     1111\r\n\
  TCP    [::]:8001              [::]:0                 LISTENING       2412\r\n\
  TCP    0.0.0.0:8001           0.0.0.0:0              LISTENING       0\r\n\
  TCP    0.0.0.0:3782           0.0.0.0:0              LISTENING       27292\r\n\
  UDP    0.0.0.0:3782           *:*                                    9999\r\n";
        assert_eq!(parse_netstat_pids(out, &[3782, 8001]), vec![2412, 27292]);
    }

    #[test]
    fn parse_netstat_pids_empty() {
        assert!(parse_netstat_pids("", &[3782]).is_empty());
    }

    #[test]
    fn taskkill_args_shape() {
        assert_eq!(
            taskkill_args(1234),
            vec!["/PID", "1234", "/T", "/F"]
        );
    }

    // 本地 TcpListener 模拟就绪/未就绪
    #[test]
    fn probe_http_accepts_3xx() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        std::thread::spawn(move || {
            if let Ok((mut s, _)) = listener.accept() {
                let _ = s.write_all(b"HTTP/1.1 307 Temporary Redirect\r\n\r\n");
            }
        });
        assert!(probe_http(port));
    }

    #[test]
    fn probe_http_rejects_5xx() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        std::thread::spawn(move || {
            if let Ok((mut s, _)) = listener.accept() {
                let _ = s.write_all(b"HTTP/1.1 500 Internal Server Error\r\n\r\n");
            }
        });
        assert!(!probe_http(port));
    }

    #[test]
    fn probe_http_refused_is_false() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        drop(listener);
        assert!(!probe_http(port));
    }
}
