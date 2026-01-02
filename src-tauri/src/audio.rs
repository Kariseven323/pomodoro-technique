//! 音频播放与音频文件管理（PRD v4：白噪音/专注音乐）。

use std::fs::File;
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::thread;

use rand::{Rng as _, SeedableRng as _};
#[cfg(windows)]
use std::io::BufReader;

#[cfg(windows)]
use rodio::{Decoder, OutputStream, OutputStreamHandle, Sink, Source as _};

use crate::app_data::{AppData, AudioSettings, CustomAudio, Phase};
use crate::errors::{AppError, AppResult};

/// 内置音效列表（注意：内置音效不可删除）。
pub fn builtin_audios() -> Vec<CustomAudio> {
    vec![
        CustomAudio {
            id: "builtin-rain".to_string(),
            name: "雨声".to_string(),
            file_name: "rain.wav".to_string(),
            builtin: true,
        },
        CustomAudio {
            id: "builtin-cafe".to_string(),
            name: "咖啡馆".to_string(),
            file_name: "cafe.wav".to_string(),
            builtin: true,
        },
        CustomAudio {
            id: "builtin-forest".to_string(),
            name: "森林".to_string(),
            file_name: "forest.wav".to_string(),
            builtin: true,
        },
        CustomAudio {
            id: "builtin-ocean".to_string(),
            name: "海浪".to_string(),
            file_name: "ocean.wav".to_string(),
            builtin: true,
        },
        CustomAudio {
            id: "builtin-white-noise".to_string(),
            name: "白噪音".to_string(),
            file_name: "white-noise.wav".to_string(),
            builtin: true,
        },
    ]
}

/// 获取音频目录下某个文件名的完整路径。
pub fn audio_file_path_in_dir(audio_dir: &Path, file_name: &str) -> PathBuf {
    audio_dir.join(file_name)
}

/// 确保音频目录存在，并在缺失时生成内置 WAV 文件。
pub fn ensure_builtin_audio_files(app: &tauri::AppHandle) -> AppResult<()> {
    let dir = crate::app_paths::app_audio_dir(app)?;
    ensure_builtin_audio_files_in_dir(&dir)
}

/// 确保音频目录存在，并在缺失时生成内置 WAV 文件（使用已解析好的目录路径）。
pub fn ensure_builtin_audio_files_in_dir(audio_dir: &Path) -> AppResult<()> {
    std::fs::create_dir_all(audio_dir)
        .map_err(|e| AppError::Invariant(format!("创建音频目录失败：{e}")))?;

    for item in builtin_audios() {
        let path = audio_dir.join(&item.file_name);
        if path.exists() {
            continue;
        }
        generate_builtin_wav(&path, &item.id)?;
    }
    Ok(())
}

/// 根据内置音效 id 生成一段 WAV（用于避免仓库内提交二进制资源文件）。
fn generate_builtin_wav(path: &Path, audio_id: &str) -> AppResult<()> {
    let sample_rate = 22_050u32;
    let seconds = 4u32;
    let len = (sample_rate * seconds) as usize;

    let mut rng = rand::rngs::StdRng::seed_from_u64(0xCAFE_F00D_u64);
    let mut samples: Vec<i16> = Vec::with_capacity(len);

    match audio_id {
        "builtin-white-noise" => {
            for _ in 0..len {
                let v: i16 = rng.gen_range(i16::MIN / 4..=i16::MAX / 4);
                samples.push(v);
            }
        }
        "builtin-rain" => {
            let mut y = 0.0f32;
            for _ in 0..len {
                let x = rng.gen_range(-1.0f32..=1.0f32);
                // 简易低通：模拟“雨声”偏柔和的质感。
                y = y * 0.98 + x * 0.02;
                samples.push((y * 12_000.0).clamp(-32_000.0, 32_000.0) as i16);
            }
        }
        "builtin-ocean" => {
            let mut phase = 0.0f32;
            let step = 2.0 * std::f32::consts::PI * 0.35 / sample_rate as f32;
            let mut y = 0.0f32;
            for _ in 0..len {
                phase += step;
                let swell = phase.sin() * 0.5 + 0.5;
                let x = rng.gen_range(-1.0f32..=1.0f32) * swell;
                y = y * 0.995 + x * 0.005;
                samples.push((y * 14_000.0).clamp(-32_000.0, 32_000.0) as i16);
            }
        }
        "builtin-forest" => {
            let mut t = 0.0f32;
            for _ in 0..len {
                t += 1.0 / sample_rate as f32;
                let chirp =
                    (2.0 * std::f32::consts::PI * (400.0 + 80.0 * (t * 0.7).sin()) * t).sin();
                let noise = rng.gen_range(-1.0f32..=1.0f32) * 0.15;
                let v = chirp * 0.15 + noise;
                samples.push((v * 18_000.0).clamp(-32_000.0, 32_000.0) as i16);
            }
        }
        "builtin-cafe" => {
            let mut y = 0.0f32;
            for _ in 0..len {
                let x = rng.gen_range(-1.0f32..=1.0f32);
                // 简易棕噪：更低频的环境底噪。
                y = (y + x * 0.02).clamp(-1.0, 1.0);
                samples.push((y * 10_000.0).clamp(-32_000.0, 32_000.0) as i16);
            }
        }
        _ => {
            return Err(AppError::Validation(format!("未知内置音效 id：{audio_id}")));
        }
    }

    write_wav_mono_16(path, sample_rate, &samples)
}

/// 写入 16-bit PCM 单声道 WAV 文件。
fn write_wav_mono_16(path: &Path, sample_rate: u32, samples: &[i16]) -> AppResult<()> {
    let mut file =
        File::create(path).map_err(|e| AppError::Invariant(format!("创建 WAV 文件失败：{e}")))?;

    let num_channels = 1u16;
    let bits_per_sample = 16u16;
    let byte_rate = sample_rate * u32::from(num_channels) * u32::from(bits_per_sample / 8);
    let block_align = num_channels * (bits_per_sample / 8);

    let data_bytes = samples.len() as u32 * 2;
    let riff_size = 36u32 + data_bytes;

    file.write_all(b"RIFF")
        .and_then(|_| file.write_all(&riff_size.to_le_bytes()))
        .and_then(|_| file.write_all(b"WAVE"))
        .and_then(|_| file.write_all(b"fmt "))
        .and_then(|_| file.write_all(&16u32.to_le_bytes())) // PCM fmt chunk size
        .and_then(|_| file.write_all(&1u16.to_le_bytes())) // audio format = PCM
        .and_then(|_| file.write_all(&num_channels.to_le_bytes()))
        .and_then(|_| file.write_all(&sample_rate.to_le_bytes()))
        .and_then(|_| file.write_all(&byte_rate.to_le_bytes()))
        .and_then(|_| file.write_all(&block_align.to_le_bytes()))
        .and_then(|_| file.write_all(&bits_per_sample.to_le_bytes()))
        .and_then(|_| file.write_all(b"data"))
        .and_then(|_| file.write_all(&data_bytes.to_le_bytes()))
        .map_err(|e| AppError::Invariant(format!("写入 WAV 头失败：{e}")))?;

    for s in samples {
        file.write_all(&s.to_le_bytes())
            .map_err(|e| AppError::Invariant(format!("写入 WAV 数据失败：{e}")))?;
    }

    Ok(())
}

/// 通过 id 在（内置 + 自定义）列表中定位音频条目。
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

    /// 暂停播放（若未播放则返回 `false`）。
    pub fn pause(&mut self) -> bool {
        self.fade_start_remaining = None;
        if let Some(sink) = &self.sink {
            sink.pause();
            return true;
        }
        false
    }

    /// 播放指定音效（会替换当前 sink），并设置为循环播放。
    pub fn play(&mut self, audio_dir: &Path, audio: &CustomAudio) -> AppResult<bool> {
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

    /// 获取当前加载的音频 id（用于上层判断是否需要切换/暂停）。
    pub fn current_audio_id(&self) -> Option<&str> {
        self.current_audio_id.as_deref()
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
            let _ = self.pause();
            return Ok(());
        }

        if settings.auto_play {
            if phase != Phase::Work || !is_running {
                let _ = self.pause();
                return Ok(());
            }

            // 自动播放：确保当前音效已开始。
            self.play_current_if_needed(audio_dir, data, settings)?;

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
            let _ = self.play(audio_dir, &audio)?;
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

    /// 暂停播放（非 Windows：始终返回 `false`，仅清理淡出状态）。
    pub fn pause(&mut self) -> bool {
        false
    }

    /// 播放指定音效（非 Windows：不实际播放，仅记录当前 id）。
    pub fn play(&mut self, _audio_dir: &Path, audio: &CustomAudio) -> AppResult<bool> {
        self.current_audio_id = Some(audio.id.clone());
        Ok(false)
    }

    /// 获取当前加载的音频 id。
    pub fn current_audio_id(&self) -> Option<&str> {
        self.current_audio_id.as_deref()
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
                let out = engine.play(&audio_dir, &audio);
                let _ = reply.send(out);
            }
            AudioCommand::Pause { reply } => {
                let out = engine.pause();
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
