// SSHTerm — Tauri 桌面 SSH/SFTP 客户端
// 入口：注册所有 Tauri 命令

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod ssh;
mod store;

use ssh::{SshSession, SftpSession, SshConfig, FileEntry};
use store::{Store, Host, HostGroup};
use std::sync::Mutex;
use tauri::State;

// ─── 应用状态 ──────────────────────────────────────────────────────────────

struct AppState {
    store: Store,
    active_sessions: Mutex<std::collections::HashMap<String, SshSession>>,
}

// ─── 命令：主机管理 ────────────────────────────────────────────────────────

#[tauri::command]
fn list_hosts(state: State<AppState>) -> Result<Vec<Host>, String> {
    state.store.list_hosts()
}

#[tauri::command]
fn get_groups(state: State<AppState>) -> Result<Vec<HostGroup>, String> {
    state.store.get_groups()
}

#[tauri::command]
fn add_host(state: State<AppState>, host: Host) -> Result<(), String> {
    state.store.add_host(&host)
}

#[tauri::command]
fn update_host(state: State<AppState>, host: Host) -> Result<(), String> {
    state.store.update_host(&host)
}

#[tauri::command]
fn delete_host(state: State<AppState>, id: String) -> Result<(), String> {
    state.store.delete_host(&id)
}

// ─── 命令：SSH 连接 ────────────────────────────────────────────────────────

#[tauri::command]
fn ssh_connect(state: State<AppState>, session_id: String, config: SshConfig) -> Result<(), String> {
    let session = SshSession::connect(&config)?;
    let mut sessions = state.active_sessions.lock().map_err(|e| e.to_string())?;
    sessions.insert(session_id, session);
    Ok(())
}

#[tauri::command]
fn ssh_disconnect(state: State<AppState>, session_id: String) -> Result<(), String> {
    let mut sessions = state.active_sessions.lock().map_err(|e| e.to_string())?;
    if let Some(session) = sessions.remove(&session_id) {
        session.disconnect()?;
    }
    Ok(())
}

#[tauri::command]
fn ssh_exec(state: State<AppState>, session_id: String, command: String) -> Result<String, String> {
    let sessions = state.active_sessions.lock().map_err(|e| e.to_string())?;
    let session = sessions.get(&session_id)
        .ok_or_else(|| "Session not found".to_string())?;
    session.exec_command(&command)
}

// ─── 命令：SFTP ─────────────────────────────────────────────────────────────

#[tauri::command]
fn sftp_list_dir(state: State<AppState>, session_id: String, path: String) -> Result<Vec<FileEntry>, String> {
    let sessions = state.active_sessions.lock().map_err(|e| e.to_string())?;
    let session = sessions.get(&session_id)
        .ok_or_else(|| "Session not found".to_string())?;

    // 提取 SFTP 会话
    let s = session.get_sftp()?;
    let sftp_session = SftpSession::new(s);
    sftp_session.list_dir(&path)
}

#[tauri::command]
fn sftp_read_file(state: State<AppState>, session_id: String, path: String) -> Result<Vec<u8>, String> {
    let sessions = state.active_sessions.lock().map_err(|e| e.to_string())?;
    let session = sessions.get(&session_id)
        .ok_or_else(|| "Session not found".to_string())?;
    let s = session.get_sftp()?;
    let sftp_session = SftpSession::new(s);
    sftp_session.read_file(&path)
}

#[tauri::command]
fn sftp_write_file(state: State<AppState>, session_id: String, path: String, data: Vec<u8>) -> Result<(), String> {
    let sessions = state.active_sessions.lock().map_err(|e| e.to_string())?;
    let session = sessions.get(&session_id)
        .ok_or_else(|| "Session not found".to_string())?;
    let s = session.get_sftp()?;
    let sftp_session = SftpSession::new(s);
    sftp_session.write_file(&path, &data)
}

#[tauri::command]
fn sftp_create_dir(state: State<AppState>, session_id: String, path: String) -> Result<(), String> {
    let sessions = state.active_sessions.lock().map_err(|e| e.to_string())?;
    let session = sessions.get(&session_id)
        .ok_or_else(|| "Session not found".to_string())?;
    let s = session.get_sftp()?;
    let sftp_session = SftpSession::new(s);
    sftp_session.create_dir(&path)
}

#[tauri::command]
fn sftp_remove(state: State<AppState>, session_id: String, path: String, is_dir: bool) -> Result<(), String> {
    let sessions = state.active_sessions.lock().map_err(|e| e.to_string())?;
    let session = sessions.get(&session_id)
        .ok_or_else(|| "Session not found".to_string())?;
    let s = session.get_sftp()?;
    let sftp_session = SftpSession::new(s);
    if is_dir {
        sftp_session.remove_dir(&path)
    } else {
        sftp_session.remove_file(&path)
    }
}

#[tauri::command]
fn sftp_rename(state: State<AppState>, session_id: String, from: String, to: String) -> Result<(), String> {
    let sessions = state.active_sessions.lock().map_err(|e| e.to_string())?;
    let session = sessions.get(&session_id)
        .ok_or_else(|| "Session not found".to_string())?;
    let s = session.get_sftp()?;
    let sftp_session = SftpSession::new(s);
    sftp_session.rename(&from, &to)
}

#[tauri::command]
fn sftp_stat(state: State<AppState>, session_id: String, path: String) -> Result<FileEntry, String> {
    let sessions = state.active_sessions.lock().map_err(|e| e.to_string())?;
    let session = sessions.get(&session_id)
        .ok_or_else(|| "Session not found".to_string())?;
    let s = session.get_sftp()?;
    let sftp_session = SftpSession::new(s);
    sftp_session.stat(&path)
}

// ─── 启动 ───────────────────────────────────────────────────────────────────

fn main() {
    env_logger::init();

    let store = Store::new().expect("Failed to initialize store");

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .manage(AppState {
            store,
            active_sessions: Mutex::new(std::collections::HashMap::new()),
        })
        .invoke_handler(tauri::generate_handler![
            list_hosts,
            get_groups,
            add_host,
            update_host,
            delete_host,
            ssh_connect,
            ssh_disconnect,
            ssh_exec,
            sftp_list_dir,
            sftp_read_file,
            sftp_write_file,
            sftp_create_dir,
            sftp_remove,
            sftp_rename,
            sftp_stat,
        ])
        .run(tauri::generate_context!())
        .expect("Error while running SSHTerm");
}
