use std::collections::HashMap;
use crate::resources::{ResourceInfo, ResourceType};

/// 内嵌资源管理器
pub struct EmbeddedResources {
    resources: HashMap<String, ResourceInfo>,
}

impl EmbeddedResources {
    /// 创建新的内嵌资源管理器
    pub fn new() -> Self {
        Self {
            resources: HashMap::new(),
        }
    }

    /// 添加资源
    pub fn add_resource(&mut self, name: String, resource: ResourceInfo) {
        self.resources.insert(name, resource);
    }

    /// 获取资源
    pub fn get_resource(&self, name: &str) -> Option<&ResourceInfo> {
        self.resources.get(name)
    }

    /// 获取所有指定类型的资源
    pub fn get_resources_by_type(&self, resource_type: &ResourceType) -> Vec<&ResourceInfo> {
        self.resources
            .values()
            .filter(|info| &info.resource_type == resource_type)
            .collect()
    }

    /// 检查资源是否存在
    pub fn has_resource(&self, name: &str) -> bool {
        self.resources.contains_key(name)
    }

    /// 获取资源数量
    pub fn resource_count(&self) -> usize {
        self.resources.len()
    }

    /// 获取所有资源名称
    pub fn resource_names(&self) -> Vec<&String> {
        self.resources.keys().collect()
    }
}

/// 构建时生成的内嵌资源数据
/// 这个模块会在 build.rs 中自动生成
pub mod generated {
    use super::*;
    
    /// 获取所有内嵌资源
    pub fn get_embedded_resources() -> EmbeddedResources {
        // 在开发模式下，返回空的资源管理器
        // 在生产构建中，这里会被 build.rs 生成的代码替换
        EmbeddedResources::new()
    }
}
