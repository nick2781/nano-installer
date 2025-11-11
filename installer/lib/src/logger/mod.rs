// 日志系统

use crate::common::Result;
use std::path::{Path, PathBuf};
use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

/// 初始化日志系统
pub fn init(log_dir: Option<&Path>, app_name: &str, is_installer: bool) -> Result<PathBuf> {
    let log_dir = log_dir.map(|p| p.to_path_buf()).unwrap_or_else(|| {
        let temp_dir = std::env::temp_dir();
        temp_dir
    });

    std::fs::create_dir_all(&log_dir)?;

    let log_type = if is_installer { "install" } else { "uninstall" };
    let timestamp = chrono::Local::now().format("%Y%m%d_%H%M%S");
    let log_filename = format!("{}_{}.log", log_type, timestamp);
    let log_path = log_dir.join(&log_filename);

    let file = std::fs::File::create(&log_path)?;

    let file_layer = fmt::layer()
        .with_writer(file)
        .with_ansi(false)
        .with_target(false)
        .with_thread_ids(false)
        .with_line_number(true);

    let env_filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("debug"));

    tracing_subscriber::registry()
        .with(env_filter)
        .with(file_layer)
        .init();

    tracing::info!(
        "=== {} v{} {} Log Started ===",
        app_name,
        env!("CARGO_PKG_VERSION"),
        if is_installer {
            "Installation"
        } else {
            "Uninstallation"
        }
    );
    tracing::info!("Log file: {:?}", log_path);
    tracing::info!(
        "System: {} {}",
        std::env::consts::OS,
        std::env::consts::ARCH
    );

    Ok(log_path)
}

/// 记录安装步骤
pub fn log_step(step: &str, status: StepStatus) {
    match status {
        StepStatus::Started => tracing::info!("[STEP] {} - Started", step),
        StepStatus::Completed => tracing::info!("[STEP] {} - Completed", step),
        StepStatus::Failed(ref err) => tracing::error!("[STEP] {} - Failed: {}", step, err),
        StepStatus::Skipped => tracing::info!("[STEP] {} - Skipped", step),
    }
}

/// 步骤状态
#[derive(Debug, Clone)]
pub enum StepStatus {
    Started,
    Completed,
    Failed(String),
    Skipped,
}
