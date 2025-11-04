// pub mod scanner;
// pub mod host;
// pub mod instance;
pub mod parameters;
// pub mod gui;
pub mod clap_integration;
pub mod trait_def;

// pub use scanner::*;
// pub use host::*;
// pub use instance::*;
pub use parameters::*;
// pub use gui::*;
pub use clap_integration::*;
pub use trait_def::*;

use thiserror::Error;

/// Plugin-related errors
#[derive(Debug, Error)]
pub enum PluginError {
    #[error("Failed to load plugin: {0}")]
    LoadFailed(String),

    #[error("Plugin initialization failed: {0}")]
    InitializationFailed(String),

    #[error("Plugin processing failed: {0}")]
    ProcessingFailed(String),

    #[error("Invalid parameter: {0}")]
    InvalidParameter(String),

    #[error("GUI operation failed: {0}")]
    GuiFailed(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Library loading error: {0}")]
    LibraryError(#[from] libloading::Error),
}

pub type PluginResult<T> = Result<T, PluginError>;
