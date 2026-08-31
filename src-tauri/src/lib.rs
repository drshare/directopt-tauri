mod compute;

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use base64::engine::general_purpose::STANDARD as BASE64;
use base64::Engine as _;
use serde::Serialize;
use tauri::{Emitter, Manager};
use tauri_plugin_log::{RotationStrategy, Target, TargetKind};
use tauri_plugin_opener::OpenerExt;

// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
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

/// 后端解析输入文件（base64 编码的 xlsx 字节），提取「项目 / 数值」参数
/// 供前端回显到参数表单，确保计算参数与传入文档一致（FR-1）
#[tauri::command]
fn parse_input_file(data: &str) -> Result<compute::input_parse::InputParsePayload, String> {
    log::info!("[parse_input_file] 收到输入文件解析请求（base64 长度 {}）", data.len());
    let bytes = BASE64
        .decode(data)
        .map_err(|e| {
            let msg = format!("输入文件数据解码失败: {e}");
            log::error!("[parse_input_file] {msg}");
            msg
        })?;
    match compute::input_parse::parse_input_xlsx(&bytes) {
        Ok(payload) => {
            log::info!(
                "[parse_input_file] 解析成功：Sheet「{}」，参数 {} 项，应用 {} 项，跳过 {} 项",
                payload.sheet_name,
                payload.values.len(),
                payload.applied_labels.len(),
                payload.skipped_labels.len()
            );
            Ok(payload)
        }
        Err(e) => {
            log::error!("[parse_input_file] 解析失败：{e}");
            Err(e)
        }
    }
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
    log::info!("[cancel_compute] 收到取消计算请求，将在下一个代间检查点终止");
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
    let started = std::time::Instant::now();
    log::info!(
        "[start_compute] 收到计算请求：目标={} 方案={} GA（代数 {} × 种群 {}）",
        params.objective,
        params.scheme,
        params.ga.generations,
        params.ga.population_size
    );

    state.cancel.store(false, Ordering::SeqCst);
    let cancel = state.cancel.clone();

    let result = tauri::async_runtime::spawn_blocking(move || {
        let emitter = app.clone();
        let on_progress = move |progress: f64, message: String| {
            let _ = emitter.emit("compute://progress", ProgressPayload { progress, message });
        };
        compute::engine::run_compute(params, curves, &on_progress, &cancel)
    })
    .await
    .map_err(|e| {
        let msg = format!("计算任务执行失败: {e}");
        log::error!("[start_compute] {msg}");
        msg
    })?;

    match &result {
        Ok(payload) => log::info!(
            "[start_compute] 计算成功：总耗时 {:.2}s，最优配置 风电 {:.1}kW / 光伏 {:.1}kW / 储能 {:.1}kWh，适应度 {:.6}",
            started.elapsed().as_secs_f64(),
            payload.best.wind_kw,
            payload.best.pv_kw,
            payload.best.ess_kwh,
            payload.best.fitness
        ),
        Err(e) => log::warn!(
            "[start_compute] 计算未完成（耗时 {:.2}s）：{e}",
            started.elapsed().as_secs_f64()
        ),
    }
    result
}

// ---- 数据文件仓库（文件管理 FR-10）：上传文件与结果文件的本地归档 ----

/// 支持的文件类别：输入文件 / 曲线文件 / 结果文件
const FILE_KINDS: [&str; 3] = ["input", "curve", "result"];

/// 数据文件元信息（列表/保存返回值）
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct StoredFileMeta {
    /// 形如 "curve/1719700000000_xxx.xlsx" 的唯一标识
    id: String,
    /// 归档时的原始文件名
    name: String,
    /// 类别：input / curve / result
    kind: String,
    /// 字节大小
    size: u64,
    /// 归档时间（Unix 毫秒）
    saved_at_ms: u64,
    /// 磁盘绝对路径（用于在系统文件管理器中定位）
    path: String,
}

/// 数据文件仓库根目录：<app_data_dir>/files/
fn files_root(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    let dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("获取应用数据目录失败: {e}"))?;
    Ok(dir.join("files"))
}

/// 校验类别合法性
fn validate_kind(kind: &str) -> Result<(), String> {
    if FILE_KINDS.contains(&kind) {
        Ok(())
    } else {
        Err(format!("非法的文件类别: {kind}"))
    }
}

/// 拆分校验文件标识 "kind/stored_name"，防止路径穿越
fn split_id(id: &str) -> Result<(String, String), String> {
    let (kind, stored) = id
        .split_once('/')
        .ok_or_else(|| "非法的文件标识".to_string())?;
    validate_kind(kind)?;
    if stored.is_empty()
        || stored.contains('/')
        || stored.contains('\\')
        || stored.contains("..")
    {
        return Err("非法的文件标识".to_string());
    }
    Ok((kind.to_string(), stored.to_string()))
}

/// 当前 Unix 毫秒时间戳
fn unix_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

/// 将前端上传/生成的文件（base64 编码）归档到应用数据目录，返回文件元信息
#[tauri::command]
fn save_data_file(
    app: tauri::AppHandle,
    name: &str,
    data: &str,
    kind: &str,
) -> Result<StoredFileMeta, String> {
    log::info!("[save_data_file] 归档文件「{name}」（类别 {kind}，base64 长度 {}）", data.len());
    validate_kind(kind)?;
    let bytes = BASE64
        .decode(data)
        .map_err(|e| {
            let msg = format!("文件数据解码失败: {e}");
            log::error!("[save_data_file] {msg}");
            msg
        })?;

    // 仅保留文件名部分，防止路径穿越
    let file_name = std::path::Path::new(name)
        .file_name()
        .ok_or_else(|| "非法的文件名".to_string())?
        .to_string_lossy()
        .into_owned();

    let dir = files_root(&app)?.join(kind);
    std::fs::create_dir_all(&dir).map_err(|e| format!("创建文件目录失败: {e}"))?;

    // 时间戳前缀避免同名覆盖
    let ts = unix_ms();
    let stored_name = format!("{ts}_{file_name}");
    let target = dir.join(&stored_name);
    std::fs::write(&target, &bytes).map_err(|e| format!("写入文件失败: {e}"))?;

    let size = std::fs::metadata(&target)
        .map(|m| m.len())
        .unwrap_or(bytes.len() as u64);
    log::info!("[save_data_file] 归档成功：{} → {}（{} 字节）", name, target.display(), size);
    Ok(StoredFileMeta {
        id: format!("{kind}/{stored_name}"),
        name: file_name,
        kind: kind.to_string(),
        size,
        saved_at_ms: ts,
        path: target.to_string_lossy().into_owned(),
    })
}

/// 列出全部归档文件（按归档时间倒序）
#[tauri::command]
fn list_data_files(app: tauri::AppHandle) -> Result<Vec<StoredFileMeta>, String> {
    let root = files_root(&app)?;
    let mut out: Vec<StoredFileMeta> = Vec::new();
    for kind in FILE_KINDS {
        let Ok(entries) = std::fs::read_dir(root.join(kind)) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            let Ok(meta) = std::fs::metadata(&path) else {
                continue;
            };
            if !meta.is_file() {
                continue;
            }
            let stored = entry.file_name().to_string_lossy().into_owned();
            // 恢复原始文件名（保存时带 "{时间戳}_" 前缀）
            let name = stored
                .split_once('_')
                .map(|(_, n)| n.to_string())
                .unwrap_or_else(|| stored.clone());
            let saved_at_ms = meta
                .modified()
                .ok()
                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|d| d.as_millis() as u64)
                .unwrap_or(0);
            out.push(StoredFileMeta {
                id: format!("{kind}/{stored}"),
                name,
                kind: kind.to_string(),
                size: meta.len(),
                saved_at_ms,
                path: path.to_string_lossy().into_owned(),
            });
        }
    }
    out.sort_by(|a, b| b.saved_at_ms.cmp(&a.saved_at_ms).then_with(|| a.id.cmp(&b.id)));
    Ok(out)
}

/// 读取归档文件内容（base64 编码），用于重新载入曲线/输入文件
#[tauri::command]
fn read_data_file(app: tauri::AppHandle, id: &str) -> Result<String, String> {
    let (kind, stored) = split_id(id)?;
    let path = files_root(&app)?.join(kind).join(stored);
    let bytes = std::fs::read(&path).map_err(|e| format!("读取文件失败: {e}"))?;
    Ok(BASE64.encode(bytes))
}

/// 删除单个归档文件
#[tauri::command]
fn delete_data_file(app: tauri::AppHandle, id: &str) -> Result<(), String> {
    let (kind, stored) = split_id(id)?;
    let path = files_root(&app)?.join(kind).join(stored);
    if path.exists() {
        std::fs::remove_file(&path).map_err(|e| format!("删除文件失败: {e}"))?;
    }
    Ok(())
}

/// 清空指定类别（缺省为全部类别）的归档文件
#[tauri::command]
fn clear_data_files(app: tauri::AppHandle, kind: Option<&str>) -> Result<(), String> {
    let root = files_root(&app)?;
    let kinds: Vec<&str> = match kind {
        Some(k) => {
            validate_kind(k)?;
            vec![k]
        }
        None => FILE_KINDS.to_vec(),
    };
    for k in kinds {
        if let Ok(entries) = std::fs::read_dir(root.join(k)) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_file() {
                    let _ = std::fs::remove_file(&path);
                }
            }
        }
    }
    Ok(())
}

/// 打开运行日志所在目录（系统文件管理器），返回日志目录路径
#[tauri::command]
fn open_log_dir(app: tauri::AppHandle) -> Result<String, String> {
    let dir = app
        .path()
        .app_log_dir()
        .map_err(|e| format!("获取日志目录失败: {e}"))?;
    std::fs::create_dir_all(&dir).map_err(|e| format!("创建日志目录失败: {e}"))?;
    app.opener()
        .open_path(dir.to_string_lossy().as_ref(), None::<&str>)
        .map_err(|e| format!("打开日志目录失败: {e}"))?;
    log::info!("[open_log_dir] 已打开日志目录：{}", dir.display());
    Ok(dir.to_string_lossy().into_owned())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(
            tauri_plugin_log::Builder::new()
                // 记录 info 及以上级别；debug 级别留给开发排查
                .level(log::LevelFilter::Info)
                // 文件按大小轮转且保留历史（避免覆盖丢失运行轨迹）
                .rotation_strategy(RotationStrategy::KeepAll)
                .max_file_size(2_000_000)
                .targets([
                    // 终端控制台（开发调试）
                    Target::new(TargetKind::Stdout),
                    // 落盘到系统日志目录（<app_log_dir>/directopt.log）
                    Target::new(TargetKind::LogDir {
                        file_name: Some("directopt".into()),
                    }),
                    // 捕获前端 console.* 输出，一并写入日志文件
                    Target::new(TargetKind::Webview),
                ])
                .build(),
        )
        .manage(AppState::default())
        .invoke_handler(tauri::generate_handler![
            save_template_file,
            parse_input_file,
            start_compute,
            cancel_compute,
            save_data_file,
            list_data_files,
            read_data_file,
            delete_data_file,
            clear_data_files,
            open_log_dir
        ])
        .setup(|app| {
            log::info!("应用启动：日志系统就绪（写入文件 directopt.log）");
            // 原生窗口标题与发布版本保持一致：「绿电直连新能源优化配置软件 V{版本}」
            // 版本取自 Cargo.toml（CARGO_PKG_VERSION），与 package.json / tauri.conf.json / git tag 同源
            if let Some(win) = app.get_webview_window("main") {
                let title = format!("绿电直连新能源优化配置软件 V{}", env!("CARGO_PKG_VERSION"));
                if let Err(e) = win.set_title(&title) {
                    log::warn!("设置窗口标题失败：{e}");
                }
            }
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
