//! Bounded frame capture on the GPU thread; PNG writing/ffmpeg run off-thread.
use crate::booth::Booth;
use std::{
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
};

#[derive(Clone, Copy)]
pub struct Options {
    pub width: u32,
    pub height: u32,
    pub fps: u32,
    pub colors: u32,
    pub dither: bool,
}

pub fn frame_count(duration: f32, fps: u32) -> Result<usize, String> {
    if !duration.is_finite()
        || duration <= 0.0
        || duration > 120.0
        || ![10, 20, 25, 50].contains(&fps)
    {
        return Err(
            "Choose an active control track (0–120 seconds) and a supported frame rate".into(),
        );
    }
    // Spine times are f32: do not turn 0.1s × 10fps into two frames due to
    // a few floating-point ulps above an exact frame boundary.
    let count = duration as f64 * fps as f64;
    Ok((count - 0.00001).ceil().max(1.0) as usize)
}

struct Capture {
    booth: Booth,
    options: Options,
    frames: usize,
    next: usize,
    pending: Option<Vec<u8>>,
    sender: mpsc::SyncSender<Vec<u8>>,
}
#[derive(Default)]
pub struct Recorder {
    capture: Option<Capture>,
    result: Option<mpsc::Receiver<Result<PathBuf, String>>>,
    cancel: Option<Arc<AtomicBool>>,
    worker: Option<std::thread::JoinHandle<()>>,
    pub progress: String,
}
impl Recorder {
    pub fn busy(&self) -> bool {
        self.result.is_some()
    }
    pub fn cancel(&mut self) {
        if let Some(cancel) = &self.cancel {
            cancel.store(true, Ordering::Relaxed);
        }
        self.capture = None;
        self.progress = "Cancelling…".into();
    }
    pub fn start(
        &mut self,
        source: &Booth,
        options: Options,
        directory: &Path,
    ) -> Result<(), String> {
        if self.busy() {
            return Err("A recording is already running".into());
        }
        let character = source
            .selected()
            .ok_or("Select a role with an active control track")?;
        let speed = character.animation.tracks[character.animation.control_track].speed;
        if speed <= 0.0 {
            return Err("Control track speed must be above zero".into());
        }
        let frames = frame_count(character.duration() / speed, options.fps)?;
        if options.width == 0
            || options.height == 0
            || options.width.max(options.height) > 1920
            || frames as u64 * options.width as u64 * options.height as u64 * 4 > 2_000_000_000
        {
            return Err(
                "Recording is too large; choose a smaller size or frame rate (2 GB safety limit)"
                    .into(),
            );
        }
        if !Command::new("ffmpeg")
            .arg("-version")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map_err(|_| "ffmpeg not found. Install ffmpeg and put it on PATH".to_string())?
            .success()
        {
            return Err("ffmpeg is unavailable".into());
        }
        let mut booth = source.recording_copy()?;
        for c in &mut booth.characters {
            let mut data = c.data();
            data.animation.paused = false;
            for track in &mut data.animation.tracks {
                track.time = 0.0;
            }
            if let Some(pet) = &mut data.animation.pet_animation {
                pet.time = 0.0;
            }
            c.restore(&data, c.placement.clone())?;
        }
        std::fs::create_dir_all(directory).map_err(|e| e.to_string())?;
        let id = crate::persistence::new_id();
        let work = directory.join(format!(".recording-{id}"));
        std::fs::create_dir(&work).map_err(|e| e.to_string())?;
        let output = directory.join(format!("KanadeSA-{id}.gif"));
        let (sender, receiver) = mpsc::sync_channel(2);
        let (done, result) = mpsc::channel();
        let cancel = Arc::new(AtomicBool::new(false));
        let flag = cancel.clone();
        self.worker = Some(std::thread::spawn(move || {
            let outcome = encode(&work, &output, options, frames, receiver, &flag);
            let _ = std::fs::remove_dir_all(&work); // Only this job's private directory.
            let _ = done.send(outcome.map(|()| output));
        }));
        self.capture = Some(Capture {
            booth,
            options,
            frames,
            next: 0,
            pending: None,
            sender,
        });
        self.result = Some(result);
        self.cancel = Some(cancel);
        self.progress = format!("Capturing 0 / {frames}");
        Ok(())
    }
    pub fn tick(&mut self) -> Option<Result<PathBuf, String>> {
        if let Some(receiver) = &self.result {
            match receiver.try_recv() {
                Ok(result) => {
                    self.capture = None;
                    self.result = None;
                    self.cancel = None;
                    if let Some(worker) = self.worker.take() {
                        let _ = worker.join();
                    }
                    self.progress.clear();
                    return Some(result);
                }
                Err(mpsc::TryRecvError::Disconnected) => {
                    self.capture = None;
                    self.result = None;
                    self.cancel = None;
                    if let Some(worker) = self.worker.take() {
                        let _ = worker.join();
                    }
                    return Some(Err("Recorder worker stopped unexpectedly".into()));
                }
                Err(mpsc::TryRecvError::Empty) => {}
            }
        }
        let Some(c) = &mut self.capture else {
            return None;
        };
        if c.pending.is_none() {
            match c.booth.capture_pixels(c.options.width, c.options.height) {
                Ok(bytes) => c.pending = Some(bytes),
                Err(error) => {
                    self.cancel();
                    return Some(Err(error));
                }
            }
        }
        match c.sender.try_send(c.pending.take().unwrap()) {
            Ok(()) => {
                c.next += 1;
                self.progress = format!("Capturing {} / {}", c.next, c.frames);
                if c.next == c.frames {
                    self.capture = None;
                    self.progress = "Encoding GIF…".into();
                } else {
                    c.booth.update(1.0 / c.options.fps as f32);
                }
            }
            Err(mpsc::TrySendError::Full(bytes)) => c.pending = Some(bytes),
            Err(mpsc::TrySendError::Disconnected(_)) => self.capture = None,
        }
        None
    }
}
impl Drop for Recorder {
    fn drop(&mut self) {
        if let Some(flag) = &self.cancel {
            flag.store(true, Ordering::Relaxed);
        }
        // Disconnect any blocked frame receiver before waiting for cleanup.
        self.capture = None;
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}
fn encode(
    work: &Path,
    output: &Path,
    options: Options,
    frames: usize,
    receiver: mpsc::Receiver<Vec<u8>>,
    cancel: &AtomicBool,
) -> Result<(), String> {
    for index in 0..frames {
        let bytes = receiver
            .recv()
            .map_err(|_| "Recording cancelled".to_string())?;
        if cancel.load(Ordering::Relaxed) {
            return Err("Recording cancelled".into());
        }
        image::save_buffer(
            work.join(format!("{index:06}.png")),
            &bytes,
            options.width,
            options.height,
            image::ColorType::Rgba8,
        )
        .map_err(|e| format!("Cannot write recording frame: {e}"))?;
    }
    if cancel.load(Ordering::Relaxed) {
        return Err("Recording cancelled".into());
    }
    // Two passes avoid buffering the entire RGBA animation in a split filter.
    let mut palette = Command::new("ffmpeg");
    palette
        .args([
            "-hide_banner",
            "-loglevel",
            "error",
            "-nostdin",
            "-y",
            "-threads",
            "1",
            "-filter_threads",
            "1",
            "-framerate",
            &options.fps.to_string(),
            "-i",
        ])
        .arg(work.join("%06d.png"))
        .args([
            "-vf",
            &format!(
                "palettegen=max_colors={}:reserve_transparent=1",
                options.colors
            ),
            "-frames:v",
            "1",
        ])
        .arg(work.join("palette.png"));
    run_ffmpeg(&mut palette, work, cancel)?;
    let filter = format!(
        "paletteuse=dither={}:alpha_threshold=128",
        if options.dither { "sierra2_4a" } else { "none" }
    );
    let mut gif = Command::new("ffmpeg");
    gif.args([
        "-hide_banner",
        "-loglevel",
        "error",
        "-nostdin",
        "-y",
        "-threads",
        "1",
        "-filter_complex_threads",
        "1",
        "-framerate",
        &options.fps.to_string(),
        "-i",
    ])
    .arg(work.join("%06d.png"))
    .arg("-i")
    .arg(work.join("palette.png"))
    .args(["-filter_complex", &filter, "-loop", "0"])
    .arg(work.join("output.gif"));
    run_ffmpeg(&mut gif, work, cancel)?;
    std::fs::rename(work.join("output.gif"), output).map_err(|e| e.to_string())
}
fn run_ffmpeg(command: &mut Command, work: &Path, cancel: &AtomicBool) -> Result<(), String> {
    let log = std::fs::File::create(work.join("ffmpeg.log")).map_err(|e| e.to_string())?;
    let mut child = command
        .stdout(Stdio::null())
        .stderr(log)
        .spawn()
        .map_err(|e| e.to_string())?;
    let encoding_started = std::time::Instant::now();
    loop {
        if cancel.load(Ordering::Relaxed) || encoding_started.elapsed().as_secs() > 300 {
            let _ = child.kill();
            let _ = child.wait();
            return Err(if cancel.load(Ordering::Relaxed) {
                "Recording cancelled"
            } else {
                "ffmpeg encoding timed out"
            }
            .into());
        }
        match child.try_wait() {
            Ok(Some(status)) => {
                if !status.success() {
                    return Err(format!(
                        "ffmpeg failed: {}",
                        std::fs::read_to_string(work.join("ffmpeg.log")).unwrap_or_default()
                    ));
                }
                break;
            }
            Ok(None) => std::thread::sleep(std::time::Duration::from_millis(50)),
            Err(error) => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(error.to_string());
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn duration_rounds_up_by_less_than_one_frame() {
        for fps in [10, 20, 25, 50] {
            for duration in [0.1, 0.333, 1.0, 3.17, 120.0] {
                let n = frame_count(duration, fps).unwrap();
                assert!(n as f32 / fps as f32 >= duration - 0.00001);
                assert!(n as f32 / fps as f32 - duration < 1.0 / fps as f32 + 0.00001);
            }
        }
        for duration in [0.0, -1.0, f32::NAN, f32::INFINITY, 121.0] {
            assert!(frame_count(duration, 20).is_err());
        }
        assert_eq!(frame_count(0.1, 10).unwrap(), 1);
    }
}
