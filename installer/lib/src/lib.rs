//! nano-installer - configuration-driven Windows installer framework
//!
//! The library powers the build CLI plus the installer and uninstaller runtime stubs.
//!
//! ## Features
//!
//! - **Modern UI**: Built with egui for a native look and feel
//! - **Configuration-driven**: JSON-based configuration with comments
//! - **XML Layouts**: Flexible UI layout system
//! - **Multi-language**: Support for 11+ languages
//! - **Windows runtime**: Windows 10 and newer
//! - **Extensible**: Rhai scripts for product-specific install behavior
//!
//! ## Quick Start
//!
//! ```bash
//! # Initialize a new project
//! nano-installer init MyApp
//!
//! # Edit configuration
//! # Edit MyApp/installer_config.json
//!
//! # Build installer
//! nano-installer build --release
//! ```
//!
//! ## Architecture
//!
//! The project is organized into several main modules:
//!
//! - `common` - Shared utilities and error handling
//! - `config` - Configuration management and validation
//! - `installer` - Core installation logic
//! - `uninstaller` - Uninstallation logic
//! - `ui` - User interface (egui-based)
//! - `layout` - XML layout parsing and rendering
//! - `i18n` - Internationalization support
//! - `resources` - Resource management and embedding

#![warn(missing_docs)]
#![warn(clippy::all)]

pub mod common;
pub mod config;
pub mod i18n;
pub mod installer;
pub mod installer_runtime;
pub mod layout;
pub mod logger;
pub mod resources;
pub mod scripting;
pub mod ui;
pub mod uninstaller;

// Re-export commonly used types
pub use common::{Error, Result};
pub use config::InstallerConfig;
pub use installer::{InstallEngine, InstallState};
pub use uninstaller::UninstallEngine;
