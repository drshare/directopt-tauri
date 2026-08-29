mod compute;

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use base64::engine::general_purpose::STANDARD as BASE64;
use base64::Engine as _;
use serde::Serialize;
use tauri::{Emitter, Manager};

// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

/// 将前端传入的模板文件（base64 编码）保存到系统下载目录，返回保存路径
#[tauri::command]
fn save_template_file(app: tauri::AppHandle, name: &str, data: &str) -> Result<String, String> {
    let bytes = BASE64
        .decode(data)
        .map_err(|e| format!("模板数据解码失败: {e}"))?;

    // 仅保留文件名部分，防止路径穿越
    let file_name = std::path::Path::new(name)
        .file_name()
        .ok_or_else(|| "非法的文件名".to_string())?;

    let dir = app
        .path()
        .download_dir()
        .map_err(|e| format!("获取下载目录失败: {e}"))?;
    std::fs::create_dir_all(&dir).map_err(|e| format!("创建下载目录失败: {e}"))?;

    let target = dir.join(file_name);
    std::fs::write(&target, bytes).map_err(|e| format!("写入模板文件失败: {e}"))?;
    Ok(target.to_string_lossy().into_owned())
}

/// 计算进度事件负载
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct ProgressPayload {
    progress: f64,
    message: String,
}

/// 应用状态：计算取消标志
#[derive(Default)]
struct AppState {
    cancel: Arc<AtomicBool>,
}

/// 取消正在进行的计算
#[tauri::command]
fn cancel_compute(state: tauri::State<AppState>) {
    state.cancel.store(true, Ordering::SeqCst);
}

/// 启动优化计算（遗传算法 + 8760h 仿真，全部在后端执行）
///
/// 计算在独立阻塞线程执行，进度通过 `compute://progress` 事件推送；
/// 命令返回最终结果负载，出错（参数非法/取消/内部异常）返回 Err。
#[tauri::command]
async fn start_compute(
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
    params: compute::ComputeParams,
    curves: compute::CurveData,
) -> Result<compute::ComputeResultPayload, String> {
    state.cancel.store(false, Ordering::SeqCst);
    let cancel = state.cancel.clone();

    tauri::async_runtime::spawn_blocking(move || {
        let emitter = app.clone();
        let on_progress = move |progress: f64, message: String| {
            let _ = emitter.emit("compute://progress", ProgressPayload { progress, message });
        };
        compute::engine::run_compute(params, curves, &on_progress, &cancel)
    })
    .await
    .map_err(|e| format!("计算任务执行失败: {e}"))?
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(AppState::default())
        .invoke_handler(tauri::generate_handler![
            greet,
            save_template_file,
            start_compute,
            cancel_compute
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
