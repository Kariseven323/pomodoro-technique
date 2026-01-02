//! 音效相关 IPC 命令：播放/暂停/音量/导入/删除/列表（PRD v4）。

use crate::app_data::CustomAudio;
use crate::commands::common::to_ipc_result;
use crate::errors::{AppError, AppResult};
use crate::state::AppState;

/// 获取音频列表（内置 + 自定义）。
#[tauri::command]
pub fn audio_list(state: tauri::State<'_, AppState>) -> Result<Vec<CustomAudio>, String> {
    to_ipc_result(audio_list_impl(&state))
}

/// 播放指定音效。
#[tauri::command]
pub fn audio_play(state: tauri::State<'_, AppState>, audio_id: String) -> Result<bool, String> {
    to_ipc_result(audio_play_impl(&state, audio_id))
}

/// 暂停播放。
#[tauri::command]
pub fn audio_pause(state: tauri::State<'_, AppState>) -> Result<bool, String> {
    to_ipc_result(audio_pause_impl(&state))
}

/// 设置音量（0-100）。
#[tauri::command]
pub fn audio_set_volume(state: tauri::State<'_, AppState>, volume: u8) -> Result<bool, String> {
    to_ipc_result(audio_set_volume_impl(&state, volume))
}

/// 导入自定义音频（复制到 `%APPDATA%/pomodoro-technique/audio/`）。
#[tauri::command]
pub fn audio_import(
    state: tauri::State<'_, AppState>,
    file_path: String,
    name: String,
) -> Result<CustomAudio, String> {
    to_ipc_result(audio_import_impl(&state, file_path, name))
}

/// 删除自定义音频（预设不可删除）。
#[tauri::command]
pub fn audio_delete(state: tauri::State<'_, AppState>, audio_id: String) -> Result<bool, String> {
    to_ipc_result(audio_delete_impl(&state, audio_id))
}

/// `audio_list` 的内部实现：确保内置资源已生成后再返回。
fn audio_list_impl(state: &AppState) -> AppResult<Vec<CustomAudio>> {
    crate::audio::ensure_builtin_audio_files_in_dir(state.audio_dir())?;
    let data = state.data_snapshot();
    let mut list = crate::audio::builtin_audios();
    list.extend(data.custom_audios.clone());
    Ok(list)
}

/// `audio_play` 的内部实现：更新当前音效 id（持久化）并开始播放。
fn audio_play_impl(state: &AppState, audio_id: String) -> AppResult<bool> {
    crate::audio::ensure_builtin_audio_files_in_dir(state.audio_dir())?;
    let audio_id = audio_id.trim().to_string();
    if audio_id.is_empty() {
        return Err(AppError::Validation("音效 id 不能为空".to_string()));
    }

    let audio = state.update_data_with(|data| {
        let Some(found) = crate::audio::find_audio_by_id(data, &audio_id) else {
            return Err(AppError::Validation("找不到指定音效".to_string()));
        };
        data.settings.audio.current_audio_id = audio_id.clone();
        Ok(found)
    })?;

    let played = state.audio_controller().play(audio)?;
    let _ = state.emit_timer_snapshot();
    state.sync_audio_with_timer()?;
    Ok(played)
}

/// `audio_pause` 的内部实现：暂停播放。
fn audio_pause_impl(state: &AppState) -> AppResult<bool> {
    state.audio_controller().pause()
}

/// `audio_set_volume` 的内部实现：持久化音量并立即应用到 sink。
fn audio_set_volume_impl(state: &AppState, volume: u8) -> AppResult<bool> {
    if volume > 100 {
        return Err(AppError::Validation("音量需在 0-100".to_string()));
    }
    state.update_data(|data| {
        data.settings.audio.volume = volume;
        Ok(())
    })?;
    let _ = state.audio_controller().set_volume(volume)?;
    let _ = state.emit_timer_snapshot();
    Ok(true)
}

/// `audio_import` 的内部实现：复制文件、写入 `custom_audios` 并返回条目。
fn audio_import_impl(state: &AppState, file_path: String, name: String) -> AppResult<CustomAudio> {
    crate::audio::ensure_builtin_audio_files_in_dir(state.audio_dir())?;
    let src = std::path::PathBuf::from(file_path.trim());
    if !src.is_file() {
        return Err(AppError::Validation(
            "选择的文件不存在或不是文件".to_string(),
        ));
    }
    let name = name.trim().to_string();
    if name.is_empty() {
        return Err(AppError::Validation("音效名称不能为空".to_string()));
    }

    let ext = src
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    let allowed = ["mp3", "wav", "ogg", "flac"];
    if !allowed.contains(&ext.as_str()) {
        return Err(AppError::Validation(
            "仅支持导入 mp3/wav/ogg/flac".to_string(),
        ));
    }

    let id = uuid::Uuid::new_v4().to_string();
    let file_name = format!("{id}.{ext}");
    let dst = state.audio_dir().join(&file_name);

    std::fs::copy(&src, &dst).map_err(|e| AppError::Invariant(format!("复制音频文件失败：{e}")))?;

    let item = CustomAudio {
        id,
        name,
        file_name,
        builtin: false,
    };

    let custom_audios = state.update_data_with(|data| {
        data.custom_audios.push(item.clone());
        Ok(data.custom_audios.clone())
    })?;
    let _ = state.audio_controller().update_custom_audios(custom_audios);
    let _ = state.emit_timer_snapshot();

    Ok(item)
}

/// `audio_delete` 的内部实现：删除条目与文件，并在必要时回退当前选中音效。
fn audio_delete_impl(state: &AppState, audio_id: String) -> AppResult<bool> {
    crate::audio::ensure_builtin_audio_files_in_dir(state.audio_dir())?;
    let audio_id = audio_id.trim().to_string();
    if audio_id.is_empty() {
        return Err(AppError::Validation("音效 id 不能为空".to_string()));
    }
    let audio_id_for_cmp = audio_id.clone();

    // 禁止删除内置音效。
    if crate::audio::builtin_audios()
        .iter()
        .any(|a| a.id == audio_id)
    {
        return Err(AppError::Validation("内置音效不可删除".to_string()));
    }

    let (file_name, should_fallback, custom_audios) = state.update_data_with(|data| {
        let Some(i) = data
            .custom_audios
            .iter()
            .position(|a| a.id == audio_id_for_cmp)
        else {
            return Err(AppError::Validation("找不到要删除的自定义音效".to_string()));
        };
        let removed = data.custom_audios.remove(i);
        let should_fallback = data.settings.audio.current_audio_id == audio_id_for_cmp;
        if should_fallback {
            data.settings.audio.current_audio_id = "builtin-white-noise".to_string();
        }
        Ok((
            removed.file_name,
            should_fallback,
            data.custom_audios.clone(),
        ))
    })?;

    let path = state.audio_dir().join(&file_name);
    let _ = std::fs::remove_file(&path);

    let _ = state.audio_controller().update_custom_audios(custom_audios);
    // 若删除了当前选中的音效，则尽量暂停并触发一次同步，避免解码/自动播放状态异常。
    if should_fallback {
        let _ = state.audio_controller().pause();
        let _ = state.sync_audio_with_timer();
    }
    let _ = state.emit_timer_snapshot();

    Ok(true)
}
