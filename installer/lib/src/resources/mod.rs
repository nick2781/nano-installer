pub mod embedded;
pub mod loader;
pub mod types;
pub mod manifest;
pub mod bundle;
pub mod payload;
pub mod runtime;

pub use embedded::EmbeddedResources;
pub use loader::ResourceLoader;
// 避免重复导出造成歧义
pub use types::*;
pub use manifest::InstallManifest;
pub use bundle::{ResourceBundle, ResourceItem, ResourceType as BundleResourceType, append_bundle_to_exe, extract_bundle_from_exe};
pub use payload::PayloadExtractor as PayloadExt;
pub use runtime::*;