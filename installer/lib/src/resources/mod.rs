pub mod bundle;
pub mod embedded;
pub mod loader;
pub mod manifest;
pub mod payload;
pub mod runtime;
pub mod types;

pub use embedded::EmbeddedResources;
pub use loader::ResourceLoader;
// 避免重复导出造成歧义
pub use bundle::{
    append_bundle_to_exe, extract_bundle_from_exe, ResourceBundle, ResourceItem,
    ResourceType as BundleResourceType,
};
pub use manifest::InstallManifest;
pub use payload::PayloadExtractor as PayloadExt;
pub use runtime::*;
pub use types::*;
