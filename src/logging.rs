//! Timestamped console logs and non-blocking daily log files.
use std::io::IsTerminal;
use tracing_appender::{
    non_blocking::WorkerGuard,
    rolling::{Builder, Rotation},
};
use tracing_subscriber::{
    EnvFilter,
    fmt::{format::Writer, time::FormatTime},
    prelude::*,
};

struct LocalTime;

impl FormatTime for LocalTime {
    fn format_time(&self, writer: &mut Writer<'_>) -> std::fmt::Result {
        write!(
            writer,
            "{}",
            chrono::Local::now().format("%Y-%m-%dT%H:%M:%S%.6f%:z")
        )
    }
}

pub fn init() -> Option<WorkerGuard> {
    let filter = match EnvFilter::try_from_default_env() {
        Ok(filter) => filter,
        Err(error) => {
            if std::env::var_os("RUST_LOG").is_some() {
                eprintln!("Invalid RUST_LOG filter: {error}; using info");
            }
            EnvFilter::new("info")
        }
    };
    let console = tracing_subscriber::fmt::layer()
        .with_timer(LocalTime)
        .with_ansi(std::io::stderr().is_terminal())
        .with_writer(std::io::stderr);
    // Create the directory before retention scans it on the first launch.
    let appender = std::fs::create_dir_all("logs")
        .map_err(|error| error.to_string())
        .and_then(|()| {
            Builder::new()
                .rotation(Rotation::DAILY)
                .filename_prefix("kanade_sa")
                .filename_suffix("log")
                .max_log_files(7)
                .build("logs")
                .map_err(|error| error.to_string())
        });
    let (file, guard, file_error) = match appender {
        Ok(appender) => {
            // Do not silently drop diagnostic messages if the writer queue is full.
            let (writer, guard) = tracing_appender::non_blocking::NonBlockingBuilder::default()
                .lossy(false)
                .finish(appender);
            (
                Some(
                    tracing_subscriber::fmt::layer()
                        .with_timer(LocalTime)
                        .with_ansi(false)
                        .with_writer(writer),
                ),
                Some(guard),
                None,
            )
        }
        Err(error) => (None, None, Some(error.to_string())),
    };
    tracing_subscriber::registry()
        .with(filter)
        .with(console)
        .with(file)
        .init();
    if let Some(error) = file_error {
        tracing::warn!("File logging unavailable: {error}");
    }
    let previous_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        tracing::error!(target: "kanade_sa::panic", "{info}");
        previous_hook(info);
    }));
    tracing::info!("Logging initialized; daily logs: logs/; retention: 7 files");
    guard
}
