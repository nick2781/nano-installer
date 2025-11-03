/// 资源打包和解包模块
/// 
/// 将项目资源（config, layouts, assets, locales, payload）打包成单一二进制文件
/// 并支持嵌入到 exe 或独立存储

use anyhow::{Context, Result, bail};
use std::collections::HashMap;
use std::fs;
use std::io::{Read, Write, Seek, SeekFrom};
use std::path::{Path, PathBuf};

/// 资源包魔数
const MAGIC: &[u8; 8] = b"NANORSRC";
const VERSION: u16 = 1;

/// 资源类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum ResourceType {
    Config = 1,
    Layout = 2,
    Asset = 3,
    Locale = 4,
    Payload = 5,
}

/// 资源项
#[derive(Debug, Clone)]
pub struct ResourceItem {
    pub resource_type: ResourceType,
    pub name: String,
    pub data: Vec<u8>,
}

/// 资源包
#[derive(Debug)]
pub struct ResourceBundle {
    pub items: Vec<ResourceItem>,
}

impl ResourceBundle {
    pub fn new() -> Self {
        Self {
            items: Vec::new(),
        }
    }

    /// 添加配置文件
    pub fn add_config(&mut self, data: Vec<u8>) -> Result<()> {
        self.items.push(ResourceItem {
            resource_type: ResourceType::Config,
            name: "installer_config.json".to_string(),
            data,
        });
        Ok(())
    }

    /// 添加 payload
    pub fn add_payload(&mut self, data: Vec<u8>) -> Result<()> {
        self.items.push(ResourceItem {
            resource_type: ResourceType::Payload,
            name: "payload.7z".to_string(),
            data,
        });
        Ok(())
    }

    /// 递归添加目录
    pub fn add_directory(&mut self, base_dir: &Path, dir_path: &Path, resource_type: ResourceType) -> Result<()> {
        for entry in fs::read_dir(dir_path)? {
            let entry = entry?;
            let path = entry.path();
            
            if path.is_file() {
                let relative_path = path.strip_prefix(base_dir)
                    .context("Failed to get relative path")?;
                let name = relative_path.to_string_lossy().replace('\\', "/");
                let data = fs::read(&path)
                    .with_context(|| format!("Failed to read file: {}", path.display()))?;
                
                self.items.push(ResourceItem {
                    resource_type,
                    name,
                    data,
                });
            } else if path.is_dir() {
                self.add_directory(base_dir, &path, resource_type)?;
            }
        }
        Ok(())
    }

    /// 打包成二进制
    pub fn pack(&self) -> Result<Vec<u8>> {
        let mut buffer = Vec::new();

        // 魔数
        buffer.write_all(MAGIC)?;
        
        // 版本号
        buffer.write_all(&VERSION.to_le_bytes())?;
        
        // 资源项数量
        buffer.write_all(&(self.items.len() as u32).to_le_bytes())?;
        
        // 目录表偏移（预留位置）
        let toc_offset_pos = buffer.len();
        buffer.write_all(&[0u8; 8])?; // u64 占位
        
        // 写入所有资源数据
        let mut toc_entries = Vec::new();
        for item in &self.items {
            let data_offset = buffer.len() as u64;
            let data_size = item.data.len() as u64;
            
            buffer.write_all(&item.data)?;
            
            toc_entries.push((
                item.resource_type,
                item.name.clone(),
                data_offset,
                data_size,
            ));
        }
        
        // 记录目录表位置
        let toc_offset = buffer.len() as u64;
        
        // 写入目录表
        for (resource_type, name, offset, size) in toc_entries {
            buffer.write_all(&[resource_type as u8])?;
            
            let name_bytes = name.as_bytes();
            buffer.write_all(&(name_bytes.len() as u16).to_le_bytes())?;
            buffer.write_all(name_bytes)?;
            
            buffer.write_all(&offset.to_le_bytes())?;
            buffer.write_all(&size.to_le_bytes())?;
        }
        
        // 回写目录表偏移
        let mut cursor = std::io::Cursor::new(&mut buffer);
        cursor.seek(SeekFrom::Start(toc_offset_pos as u64))?;
        cursor.write_all(&toc_offset.to_le_bytes())?;
        
        // CRC32 校验（整个包）
        let crc = crc32fast::hash(&buffer);
        cursor.seek(SeekFrom::End(0))?;
        cursor.write_all(&crc.to_le_bytes())?;
        
        Ok(buffer)
    }

    /// 从二进制解包
    pub fn unpack(data: &[u8]) -> Result<Self> {
        let mut cursor = std::io::Cursor::new(data);
        
        // 验证魔数
        let mut magic = [0u8; 8];
        cursor.read_exact(&mut magic)?;
        if &magic != MAGIC {
            bail!("Invalid resource bundle: magic number mismatch");
        }
        
        // 读取版本号
        let mut version_bytes = [0u8; 2];
        cursor.read_exact(&mut version_bytes)?;
        let version = u16::from_le_bytes(version_bytes);
        if version != VERSION {
            bail!("Unsupported resource bundle version: {}", version);
        }
        
        // 读取资源项数量
        let mut count_bytes = [0u8; 4];
        cursor.read_exact(&mut count_bytes)?;
        let count = u32::from_le_bytes(count_bytes);
        
        // 读取目录表偏移
        let mut toc_offset_bytes = [0u8; 8];
        cursor.read_exact(&mut toc_offset_bytes)?;
        let toc_offset = u64::from_le_bytes(toc_offset_bytes);
        
        // 跳转到目录表
        cursor.seek(SeekFrom::Start(toc_offset))?;
        
        // 读取目录表
        let mut items = Vec::new();
        for _ in 0..count {
            let mut type_byte = [0u8; 1];
            cursor.read_exact(&mut type_byte)?;
            let resource_type = match type_byte[0] {
                1 => ResourceType::Config,
                2 => ResourceType::Layout,
                3 => ResourceType::Asset,
                4 => ResourceType::Locale,
                5 => ResourceType::Payload,
                _ => bail!("Unknown resource type: {}", type_byte[0]),
            };
            
            let mut name_len_bytes = [0u8; 2];
            cursor.read_exact(&mut name_len_bytes)?;
            let name_len = u16::from_le_bytes(name_len_bytes) as usize;
            
            let mut name_bytes = vec![0u8; name_len];
            cursor.read_exact(&mut name_bytes)?;
            let name = String::from_utf8(name_bytes)?;
            
            let mut offset_bytes = [0u8; 8];
            cursor.read_exact(&mut offset_bytes)?;
            let offset = u64::from_le_bytes(offset_bytes);
            
            let mut size_bytes = [0u8; 8];
            cursor.read_exact(&mut size_bytes)?;
            let size = u64::from_le_bytes(size_bytes);
            
            // 读取资源数据
            let data = data[offset as usize..(offset + size) as usize].to_vec();
            
            items.push(ResourceItem {
                resource_type,
                name,
                data,
            });
        }
        
        Ok(Self { items })
    }

    /// 获取指定资源
    pub fn get(&self, name: &str) -> Option<&ResourceItem> {
        self.items.iter().find(|item| item.name == name)
    }

    /// 获取指定类型的所有资源
    pub fn get_by_type(&self, resource_type: ResourceType) -> Vec<&ResourceItem> {
        self.items.iter()
            .filter(|item| item.resource_type == resource_type)
            .collect()
    }
}

/// 将资源包追加到 exe 文件
pub fn append_bundle_to_exe(exe_path: &Path, bundle_data: &[u8]) -> Result<()> {
    let mut file = fs::OpenOptions::new()
        .append(true)
        .open(exe_path)?;
    
    // 写入资源包数据
    file.write_all(bundle_data)?;
    
    // 写入资源包大小（用于从 exe 末尾反向读取）
    let bundle_size = bundle_data.len() as u64;
    file.write_all(&bundle_size.to_le_bytes())?;
    
    // 写入尾部标记
    file.write_all(MAGIC)?;
    
    Ok(())
}

/// 从 exe 文件读取嵌入的资源包
pub fn extract_bundle_from_exe(exe_path: &Path) -> Result<ResourceBundle> {
    let mut file = fs::File::open(exe_path)?;
    let file_size = file.metadata()?.len();
    
    // 读取尾部标记（最后 8 字节）
    file.seek(SeekFrom::End(-8))?;
    let mut magic = [0u8; 8];
    file.read_exact(&mut magic)?;
    
    if &magic != MAGIC {
        bail!("No embedded resource bundle found in exe");
    }
    
    // 读取资源包大小（倒数第 9-16 字节）
    file.seek(SeekFrom::End(-16))?;
    let mut size_bytes = [0u8; 8];
    file.read_exact(&mut size_bytes)?;
    let bundle_size = u64::from_le_bytes(size_bytes);
    
    // 读取资源包数据
    let bundle_offset = file_size - bundle_size - 16;
    file.seek(SeekFrom::Start(bundle_offset))?;
    
    let mut bundle_data = vec![0u8; bundle_size as usize];
    file.read_exact(&mut bundle_data)?;
    
    ResourceBundle::unpack(&bundle_data)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pack_unpack() {
        let mut bundle = ResourceBundle::new();
        bundle.add_config(b"test config".to_vec()).unwrap();
        
        let packed = bundle.pack().unwrap();
        let unpacked = ResourceBundle::unpack(&packed).unwrap();
        
        assert_eq!(unpacked.items.len(), 1);
        assert_eq!(unpacked.items[0].name, "installer_config.json");
        assert_eq!(unpacked.items[0].data, b"test config");
    }
}

