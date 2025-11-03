use serde::{Deserialize, Serialize};

/// 安装清单
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstallManifest {
    pub version: String,
    pub product_name: String,
    pub publisher: String,
    pub install_path: String,
    pub files: Vec<FileEntry>,
    pub shortcuts: Vec<ShortcutEntry>,
    pub registry_entries: Vec<RegistryEntry>,
}

impl InstallManifest {
    pub fn new(
        version: String,
        product_name: String,
        publisher: String,
        install_path: String,
    ) -> Self {
        Self {
            version,
            product_name,
            publisher,
            install_path,
            files: Vec::new(),
            shortcuts: Vec::new(),
            registry_entries: Vec::new(),
        }
    }

    pub fn save(&self, path: &std::path::Path) -> Result<(), Box<dyn std::error::Error>> {
        let content = serde_json::to_string_pretty(self)?;
        std::fs::write(path, content)?;
        Ok(())
    }
}

/// 文件条目
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileEntry {
    pub source: String,
    pub destination: String,
    pub size: u64,
    pub checksum: Option<String>,
}

/// 快捷方式条目
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShortcutEntry {
    pub name: String,
    pub target: String,
    pub arguments: Option<String>,
    pub icon: Option<String>,
    pub location: ShortcutLocation,
}

/// 快捷方式位置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ShortcutLocation {
    Desktop,
    StartMenu,
    Both,
}

/// 注册表条目
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegistryEntry {
    pub key: String,
    pub value_name: String,
    pub value_data: String,
    pub value_type: RegistryValueType,
}

/// 注册表值类型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RegistryValueType {
    String,
    DWord,
    Binary,
}

/// 卸载清单
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UninstallManifest {
    pub version: String,
    pub product_name: String,
    pub install_path: String,
    pub locale: String,
    pub files_to_remove: Vec<String>,
    pub directories_to_remove: Vec<String>,
    pub registry_keys_to_remove: Vec<String>,
    pub shortcuts_to_remove: Vec<String>,
}

impl UninstallManifest {
    pub fn load(path: &std::path::Path) -> Result<Self, Box<dyn std::error::Error>> {
        let content = std::fs::read_to_string(path)?;
        let manifest: UninstallManifest = serde_json::from_str(&content)?;
        Ok(manifest)
    }
}

/// 载荷提取器
pub struct PayloadExtractor {
    pub manifest: InstallManifest,
}

impl PayloadExtractor {
    pub fn new(manifest: InstallManifest) -> Self {
        Self { manifest }
    }

    /// 提取7z文件到指定目录
    pub fn extract_7z_to_dir(
        _payload_data: &[u8],
        _target_dir: &std::path::Path,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // 这里实现7z解压逻辑
        // 目前是占位符
        Ok(())
    }

    /// 提取文件到指定目录
    pub fn extract_to(&self, _target_dir: &std::path::Path) -> Result<(), Box<dyn std::error::Error>> {
        // 这里实现文件提取逻辑
        // 目前是占位符
        Ok(())
    }

    /// 获取文件列表
    pub fn get_files(&self) -> &[FileEntry] {
        &self.manifest.files
    }

    /// 获取总大小
    pub fn get_total_size(&self) -> u64 {
        self.manifest.files.iter().map(|f| f.size).sum()
    }
}