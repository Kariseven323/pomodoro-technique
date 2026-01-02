//! 音频播放与音频文件管理（PRD v4：白噪音/专注音乐）。

use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::thread;

#[cfg(windows)]
use std::fs::File;
#[cfg(windows)]
use std::io::BufReader;

#[cfg(windows)]
use rodio::{Decoder, OutputStream, OutputStreamHandle, Sink, Source as _};

use crate::app_data::{AppData, AudioSettings, CustomAudio, Phase};
use crate::errors::{AppError, AppResult};

/// 前端事件：音频库变更（导入/删除后推送给前端，用于刷新下拉框列表）。
pub const EVENT_AUDIO_LIBRARY_CHANGED: &str = "pomodoro://audio_library_changed";

/// 音效播放模式：自动跟随番茄（`autoPlay`）或手动试听。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AudioPlayMode {
    /// 自动模式：由计时器状态驱动播放/暂停/淡出。
    Auto,
    /// 手动模式：用户点击播放/暂停后进入，计时器不再强制暂停（用于未开始计时的试听）。
    Manual,
}

/// 预设音效列表（v4 不提供预设音效，因此为空）。
pub fn builtin_audios() -> Vec<CustomAudio> {
    vec![]
}

/// Windows：获取音频目录下某个文件名的完整路径。
#[cfg(windows)]
pub fn audio_file_path_in_dir(audio_dir: &Path, file_name: &str) -> PathBuf {
    audio_dir.join(file_name)
}

/// 确保音频目录存在（使用已解析好的目录路径；v4 不再生成预设音频文件）。
pub fn ensure_builtin_audio_files_in_dir(audio_dir: &Path) -> AppResult<()> {
    std::fs::create_dir_all(audio_dir)
        .map_err(|e| AppError::Invariant(format!("创建音频目录失败：{e}")))?;
    Ok(())
}

/// 清理旧版本生成的“预设占位符音效文件”（仅 best-effort，不会影响启动）。
pub fn cleanup_legacy_builtin_audio_files(audio_dir: &Path) -> AppResult<u32> {
    let candidates = [
        "rain.wav",
        "cafe.wav",
        "forest.wav",
        "ocean.wav",
        "white-noise.wav",
    ];
    let mut removed = 0u32;
    for name in candidates {
        let path = audio_dir.join(name);
        match std::fs::remove_file(&path) {
            Ok(()) => removed = removed.saturating_add(1),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => {
                return Err(AppError::Invariant(format!(
                    "清理旧预设音效文件失败：{}（{e}）",
                    path.to_string_lossy()
                )));
            }
        }
    }
    Ok(removed)
}

/// 通过 id 在音频列表中定位音频条目（v4 仅自定义音频）。
pub fn find_audio_by_id(data: &AppData, audio_id: &str) -> Option<CustomAudio> {
    let builtin = builtin_audios().into_iter().find(|a| a.id == audio_id);
    if builtin.is_some() {
        return builtin;
    }
    data.custom_audios
        .iter()
        .find(|a| a.id == audio_id)
        .cloned()
}

/// 音频播放引擎：封装 `rodio` 输出与淡出逻辑。
#[cfg(windows)]
pub struct AudioEngine {
    /// `rodio` 输出流（必须保持存活，否则 sink 无声）。
    stream: Option<OutputStream>,
    /// `rodio` 输出句柄（创建 sink 使用）。
    handle: Option<OutputStreamHandle>,
    /// 当前 sink（用于播放/暂停/调音量）。
    sink: Option<Sink>,
    /// 当前正在加载的音频 id。
    current_audio_id: Option<String>,
    /// 当前播放模式（自动/手动）。
    play_mode: AudioPlayMode,
    /// 目标音量（0.0-1.0）。
    target_volume: f32,
    /// 淡出开始时的 `remainingSeconds`（固定为 5）。
    fade_start_remaining: Option<u64>,
    /// 淡出开始时的音量（用于线性插值）。
    fade_start_volume: f32,
}

#[cfg(windows)]
impl std::fmt::Debug for AudioEngine {
    /// 格式化调试信息（避免 `rodio` 类型缺少 `Debug` 导致编译失败）。
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AudioEngine")
            .field("current_audio_id", &self.current_audio_id)
            .field("play_mode", &self.play_mode)
            .field("target_volume", &self.target_volume)
            .field("fade_start_remaining", &self.fade_start_remaining)
            .field("fade_start_volume", &self.fade_start_volume)
            .finish()
    }
}

#[cfg(not(windows))]
#[derive(Debug)]
pub struct AudioEngine {
    /// 当前“逻辑选中”的音频 id（用于 UI 状态同步）。
    current_audio_id: Option<String>,
    /// 当前播放模式（自动/手动）。
    play_mode: AudioPlayMode,
    /// 目标音量（0.0-1.0）。
    target_volume: f32,
}

#[cfg(windows)]
impl Default for AudioEngine {
    /// 默认：未初始化输出设备，未加载任何音频。
    fn default() -> Self {
        Self {
            stream: None,
            handle: None,
            sink: None,
            current_audio_id: None,
            play_mode: AudioPlayMode::Auto,
            target_volume: 0.6,
            fade_start_remaining: None,
            fade_start_volume: 0.6,
        }
    }
}

#[cfg(not(windows))]
impl Default for AudioEngine {
    /// 默认：无播放能力，但保留必要的状态字段，避免非 Windows 环境编译失败。
    fn default() -> Self {
        Self {
            current_audio_id: None,
            play_mode: AudioPlayMode::Auto,
            target_volume: 0.6,
        }
    }
}

#[cfg(windows)]
impl AudioEngine {
    /// 创建音频引擎（默认延迟初始化输出设备，避免启动时因设备异常阻塞）。
    pub fn new() -> Self {
        Self::default()
    }

    /// 设置目标音量（0-100），并在非淡出阶段立即应用到 sink。
    pub fn set_volume(&mut self, volume: u8) {
        self.target_volume = (volume.min(100) as f32) / 100.0;
        if self.fade_start_remaining.is_none() {
            if let Some(sink) = &self.sink {
                sink.set_volume(self.target_volume);
            }
        }
    }

    /// 内部暂停实现：不会变更播放模式（用于自动同步逻辑）。
    fn pause_inner(&mut self) -> bool {
        self.fade_start_remaining = None;
        if let Some(sink) = &self.sink {
            sink.pause();
            return true;
        }
        false
    }

    /// 手动暂停播放：进入手动模式（用于“未开始计时的试听”或用户主动暂停）。
    pub fn pause_manual(&mut self) -> bool {
        self.play_mode = AudioPlayMode::Manual;
        self.pause_inner()
    }

    /// 播放指定音效的内部实现：会替换当前 sink，并设置为循环播放。
    fn play_inner(&mut self, audio_dir: &Path, audio: &CustomAudio) -> AppResult<bool> {
        self.fade_start_remaining = None;
        self.ensure_stream()?;

        let path = audio_file_path_in_dir(audio_dir, &audio.file_name);
        let file = File::open(&path).map_err(|e| {
            AppError::Validation(format!(
                "无法打开音频文件：{}（{e}）",
                path.to_string_lossy()
            ))
        })?;
        let source = Decoder::new(BufReader::new(file))
            .map_err(|e| AppError::Validation(format!("音频解码失败：{e}")))?;

        let handle = self
            .handle
            .as_ref()
            .ok_or_else(|| AppError::Invariant("音频输出句柄缺失".to_string()))?;
        let sink = Sink::try_new(handle)
            .map_err(|e| AppError::Invariant(format!("创建音频 sink 失败：{e}")))?;
        sink.set_volume(self.target_volume);
        sink.append(source.repeat_infinite());
        sink.play();

        self.sink = Some(sink);
        self.current_audio_id = Some(audio.id.clone());
        Ok(true)
    }

    /// 手动播放：进入手动模式并开始播放（用于 UI 的播放按钮）。
    pub fn play_manual(&mut self, audio_dir: &Path, audio: &CustomAudio) -> AppResult<bool> {
        self.play_mode = AudioPlayMode::Manual;
        self.play_inner(audio_dir, audio)
    }

    /// 自动播放：进入自动模式并开始播放（用于 `autoPlay` 同步逻辑）。
    fn play_auto(&mut self, audio_dir: &Path, audio: &CustomAudio) -> AppResult<bool> {
        self.play_mode = AudioPlayMode::Auto;
        self.play_inner(audio_dir, audio)
    }

    /// 根据当前计时器状态同步音效：自动播放/暂停 + 结束前淡出。
    pub fn sync_with_timer(
        &mut self,
        data: &AppData,
        audio_dir: &Path,
        settings: &AudioSettings,
        phase: Phase,
        is_running: bool,
        remaining_seconds: u64,
    ) -> AppResult<()> {
        ensure_builtin_audio_files_in_dir(audio_dir)?;
        self.set_volume(settings.volume);

        if !settings.enabled {
            let _ = self.pause_inner();
            return Ok(());
        }

        if settings.auto_play {
            if phase != Phase::Work || !is_running {
                // 仅在“自动模式”下强制暂停；手动模式用于试听，不应被计时器状态立刻打断。
                if self.play_mode == AudioPlayMode::Auto {
                    let _ = self.pause_inner();
                }
                return Ok(());
            }

            // 自动播放：确保当前音效已开始。
            if self.play_mode == AudioPlayMode::Auto {
                self.play_current_if_needed(audio_dir, data, settings)?;
            }

            // PRD v4：计时结束前 5 秒开始淡出，淡出时长 3 秒，淡出后自动暂停。
            self.maybe_fade_out(remaining_seconds);
            self.apply_fade_if_needed(remaining_seconds);
        }

        Ok(())
    }

    /// 在自动播放模式下，确保当前选中音效已加载并处于播放态。
    fn play_current_if_needed(
        &mut self,
        audio_dir: &Path,
        data: &AppData,
        settings: &AudioSettings,
    ) -> AppResult<()> {
        let current = settings.current_audio_id.trim();
        if current.is_empty() {
            return Ok(());
        }
        if self.current_audio_id.as_deref() == Some(current) {
            if let Some(sink) = &self.sink {
                sink.play();
            }
            return Ok(());
        }

        if let Some(audio) = find_audio_by_id(data, current) {
            let _ = self.play_auto(audio_dir, &audio)?;
        }
        Ok(())
    }

    /// 在“剩余 5 秒”时进入淡出状态（若尚未淡出）。
    fn maybe_fade_out(&mut self, remaining_seconds: u64) {
        if remaining_seconds != 5 {
            return;
        }
        if self.fade_start_remaining.is_some() {
            return;
        }
        if self.sink.is_none() {
            return;
        }
        self.fade_start_remaining = Some(5);
        self.fade_start_volume = self.target_volume;
    }

    /// 在淡出状态下根据剩余秒数调整音量，并在淡出结束后暂停。
    fn apply_fade_if_needed(&mut self, remaining_seconds: u64) {
        let Some(start) = self.fade_start_remaining else {
            return;
        };
        let Some(sink) = &self.sink else {
            self.fade_start_remaining = None;
            return;
        };

        let elapsed = start.saturating_sub(remaining_seconds);
        if elapsed >= 3 {
            sink.pause();
            sink.set_volume(self.target_volume);
            self.fade_start_remaining = None;
            return;
        }

        let p = 1.0 - (elapsed as f32 / 3.0);
        sink.set_volume((self.fade_start_volume * p).max(0.0));
    }

    /// 初始化 `rodio` 输出（按需执行）。
    fn ensure_stream(&mut self) -> AppResult<()> {
        if self.stream.is_some() && self.handle.is_some() {
            return Ok(());
        }
        match OutputStream::try_default() {
            Ok((stream, handle)) => {
                self.stream = Some(stream);
                self.handle = Some(handle);
                Ok(())
            }
            Err(e) => Err(AppError::Invariant(format!("初始化音频输出失败：{e}"))),
        }
    }
}

#[cfg(not(windows))]
impl AudioEngine {
    /// 创建音频引擎（非 Windows：仅保留状态，不实际播放）。
    pub fn new() -> Self {
        Self::default()
    }

    /// 设置目标音量（0-100）。
    pub fn set_volume(&mut self, volume: u8) {
        self.target_volume = (volume.min(100) as f32) / 100.0;
    }

    /// 内部暂停实现：不会变更播放模式（用于自动同步逻辑）。
    fn pause_inner(&mut self) -> bool {
        false
    }

    /// 手动暂停播放：进入手动模式（非 Windows：不实际播放，仅记录模式变化）。
    pub fn pause_manual(&mut self) -> bool {
        self.play_mode = AudioPlayMode::Manual;
        self.pause_inner()
    }

    /// 播放指定音效的内部实现（非 Windows：不实际播放，仅记录当前 id）。
    fn play_inner(&mut self, _audio_dir: &Path, audio: &CustomAudio) -> AppResult<bool> {
        self.current_audio_id = Some(audio.id.clone());
        Ok(false)
    }

    /// 手动播放：进入手动模式（非 Windows：不实际播放，仅记录当前 id）。
    pub fn play_manual(&mut self, audio_dir: &Path, audio: &CustomAudio) -> AppResult<bool> {
        self.play_mode = AudioPlayMode::Manual;
        self.play_inner(audio_dir, audio)
    }

    /// 同步计时器状态（非 Windows：不实际播放，始终返回成功）。
    pub fn sync_with_timer(
        &mut self,
        _data: &AppData,
        audio_dir: &Path,
        settings: &AudioSettings,
        _phase: Phase,
        _is_running: bool,
        _remaining_seconds: u64,
    ) -> AppResult<()> {
        ensure_builtin_audio_files_in_dir(audio_dir)?;
        self.set_volume(settings.volume);
        Ok(())
    }
}

/// 音频控制命令：通过线程消息驱动内部 `AudioEngine`（避免将非 Send 类型放入 `AppState`）。
#[derive(Debug)]
pub enum AudioCommand {
    /// 更新自定义音频列表（用于 `autoPlay` 时按 id 解析文件）。
    UpdateCustomAudios {
        /// 自定义音频列表快照。
        custom_audios: Vec<CustomAudio>,
    },
    /// 设置音量（0-100）。
    SetVolume {
        /// 音量（0-100）。
        volume: u8,
        /// 响应通道：返回是否成功应用。
        reply: mpsc::Sender<AppResult<bool>>,
    },
    /// 播放指定音效（按条目）。
    Play {
        /// 要播放的音频条目（内置或自定义）。
        audio: CustomAudio,
        /// 响应通道：返回是否开始播放（非 Windows 可能为 `false`）。
        reply: mpsc::Sender<AppResult<bool>>,
    },
    /// 暂停播放。
    Pause {
        /// 响应通道：返回是否发生了暂停（非 Windows 为 `false`）。
        reply: mpsc::Sender<AppResult<bool>>,
    },
    /// 同步计时器状态（用于 autoPlay 与淡出）。
    SyncTimer {
        /// 音效设置快照。
        settings: AudioSettings,
        /// 当前阶段。
        phase: Phase,
        /// 是否运行中。
        is_running: bool,
        /// 剩余秒数。
        remaining_seconds: u64,
    },
}

/// 音频控制器：可安全放入 `AppState`，并通过后台线程驱动真实播放。
#[derive(Debug, Clone)]
pub struct AudioController {
    /// 音频目录路径（包含内置生成与自定义导入）。
    audio_dir: PathBuf,
    /// 命令发送端。
    tx: mpsc::Sender<AudioCommand>,
}

impl AudioController {
    /// 创建音频控制器并启动后台线程。
    pub fn new(audio_dir: PathBuf) -> AppResult<Self> {
        ensure_builtin_audio_files_in_dir(&audio_dir)?;
        let (tx, rx) = mpsc::channel::<AudioCommand>();
        let dir_for_thread = audio_dir.clone();
        thread::spawn(move || {
            run_audio_thread(dir_for_thread, rx);
        });
        Ok(Self { audio_dir, tx })
    }

    /// 更新自定义音频列表快照（供 autoPlay 按 id 解析）。
    pub fn update_custom_audios(&self, custom_audios: Vec<CustomAudio>) -> AppResult<()> {
        self.tx
            .send(AudioCommand::UpdateCustomAudios { custom_audios })
            .map_err(|_| AppError::Invariant("音频线程已退出".to_string()))?;
        Ok(())
    }

    /// 播放指定音频条目。
    pub fn play(&self, audio: CustomAudio) -> AppResult<bool> {
        let (reply_tx, reply_rx) = mpsc::channel();
        self.tx
            .send(AudioCommand::Play {
                audio,
                reply: reply_tx,
            })
            .map_err(|_| AppError::Invariant("音频线程已退出".to_string()))?;
        reply_rx
            .recv()
            .map_err(|_| AppError::Invariant("音频线程未返回结果".to_string()))?
    }

    /// 暂停播放。
    pub fn pause(&self) -> AppResult<bool> {
        let (reply_tx, reply_rx) = mpsc::channel();
        self.tx
            .send(AudioCommand::Pause { reply: reply_tx })
            .map_err(|_| AppError::Invariant("音频线程已退出".to_string()))?;
        reply_rx
            .recv()
            .map_err(|_| AppError::Invariant("音频线程未返回结果".to_string()))?
    }

    /// 设置音量（0-100）。
    pub fn set_volume(&self, volume: u8) -> AppResult<bool> {
        let (reply_tx, reply_rx) = mpsc::channel();
        self.tx
            .send(AudioCommand::SetVolume {
                volume,
                reply: reply_tx,
            })
            .map_err(|_| AppError::Invariant("音频线程已退出".to_string()))?;
        reply_rx
            .recv()
            .map_err(|_| AppError::Invariant("音频线程未返回结果".to_string()))?
    }

    /// 同步计时器状态（autoPlay/淡出）。
    pub fn sync_timer(
        &self,
        settings: AudioSettings,
        phase: Phase,
        is_running: bool,
        remaining_seconds: u64,
    ) -> AppResult<()> {
        self.tx
            .send(AudioCommand::SyncTimer {
                settings,
                phase,
                is_running,
                remaining_seconds,
            })
            .map_err(|_| AppError::Invariant("音频线程已退出".to_string()))?;
        Ok(())
    }

    /// 获取音频目录路径（供导入/删除等命令复用）。
    pub fn audio_dir(&self) -> &Path {
        &self.audio_dir
    }
}

/// 音频线程主循环：串行处理命令，持有 `AudioEngine` 的所有权（避免 `Send` 约束问题）。
fn run_audio_thread(audio_dir: PathBuf, rx: mpsc::Receiver<AudioCommand>) {
    let mut engine = AudioEngine::new();
    let mut data = AppData::default();
    // 音频线程不应持久化任何 AppData，只保留 `custom_audios` 以便按 id 解析。
    data.custom_audios = Vec::new();

    for cmd in rx {
        match cmd {
            AudioCommand::UpdateCustomAudios { custom_audios } => {
                data.custom_audios = custom_audios;
            }
            AudioCommand::SetVolume { volume, reply } => {
                engine.set_volume(volume);
                let _ = reply.send(Ok(true));
            }
            AudioCommand::Play { audio, reply } => {
                let out = engine.play_manual(&audio_dir, &audio);
                let _ = reply.send(out);
            }
            AudioCommand::Pause { reply } => {
                let out = engine.pause_manual();
                let _ = reply.send(Ok(out));
            }
            AudioCommand::SyncTimer {
                settings,
                phase,
                is_running,
                remaining_seconds,
            } => {
                let _ = engine.sync_with_timer(
                    &data,
                    &audio_dir,
                    &settings,
                    phase,
                    is_running,
                    remaining_seconds,
                );
            }
        }
    }
}
