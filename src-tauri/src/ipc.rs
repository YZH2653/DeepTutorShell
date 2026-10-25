use crate::error::AppError;
use crate::service::{self, ServiceState, StatusPayload};
use tauri::State;

// 查询服务状态快照（防事件早发丢失）
#[tauri::command]
pub fn get_status(state: State<'_, ServiceState>) -> StatusPayload {
    state.last_status()
}

// 重试启动（作废旧轮询）
#[tauri::command]
pub fn retry_start(app: tauri::AppHandle) -> Result<(), AppError> {
    service::start_coordinator(app);
    Ok(())
}

// 打开服务日志
#[tauri::command]
pub fn open_service_log(state: State<'_, ServiceState>) -> Result<(), AppError> {
    let path = state
        .log_path()
        .ok_or_else(|| AppError::new("服务日志尚未生成"))?;
    if !path.exists() {
        return Err(AppError::new("服务日志尚未生成"));
    }
    tauri_plugin_opener::open_path(path, None::<&str>)
        .map_err(|e| AppError::new(format!("打开日志失败：{e}")))?;
    Ok(())
}
