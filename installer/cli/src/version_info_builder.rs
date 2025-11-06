/// 完整的 Windows VS_VERSION_INFO 资源构建器
/// 
/// 参考: https://learn.microsoft.com/en-us/windows/win32/menurc/vs-versioninfo
/// 
/// 结构层次：
/// VS_VERSION_INFO
///   ├─ VS_FIXEDFILEINFO (value)
///   ├─ StringFileInfo (child)
///   │   └─ StringTable (child)
///   │       └─ String (children)
///   └─ VarFileInfo (child)
///       └─ Var (child)

use std::io::Write;

pub struct VersionInfoBuilder {
    file_version: (u16, u16, u16, u16),
    product_version: (u16, u16, u16, u16),
    strings: Vec<(String, String)>,
}

impl VersionInfoBuilder {
    pub fn new() -> Self {
        Self {
            file_version: (1, 0, 0, 0),
            product_version: (1, 0, 0, 0),
            strings: Vec::new(),
        }
    }
    
    pub fn file_version(mut self, major: u16, minor: u16, patch: u16, build: u16) -> Self {
        self.file_version = (major, minor, patch, build);
        self
    }
    
    pub fn product_version(mut self, major: u16, minor: u16, patch: u16, build: u16) -> Self {
        self.product_version = (major, minor, patch, build);
        self
    }
    
    pub fn add_string(mut self, key: &str, value: &str) -> Self {
        self.strings.push((key.to_string(), value.to_string()));
        self
    }
    
    pub fn build(self) -> Vec<u8> {
        let mut buffer = Vec::new();
        
        // 构建完整的 VS_VERSION_INFO 结构
        self.write_version_info(&mut buffer);
        
        buffer
    }
    
    fn write_version_info(&self, buffer: &mut Vec<u8>) {
        let start_pos = buffer.len();
        
        // 预留 wLength 位置
        buffer.write_all(&0u16.to_le_bytes()).unwrap();
        
        // wValueLength = sizeof(VS_FIXEDFILEINFO) = 52
        buffer.write_all(&52u16.to_le_bytes()).unwrap();
        
        // wType = 0 (binary)
        buffer.write_all(&0u16.to_le_bytes()).unwrap();
        
        // szKey = "VS_VERSION_INFO"
        write_wstring(buffer, "VS_VERSION_INFO");
        
        // 对齐到 DWORD 边界
        pad_to_dword(buffer);
        
        // Value = VS_FIXEDFILEINFO
        self.write_fixed_file_info(buffer);
        
        // 对齐到 DWORD 边界
        pad_to_dword(buffer);
        
        // Children
        self.write_string_file_info(buffer);
        pad_to_dword(buffer);
        
        self.write_var_file_info(buffer);
        pad_to_dword(buffer);
        
        // 回填 wLength
        let total_length = buffer.len() - start_pos;
        let length_bytes = (total_length as u16).to_le_bytes();
        buffer[start_pos] = length_bytes[0];
        buffer[start_pos + 1] = length_bytes[1];
    }
    
    fn write_fixed_file_info(&self, buffer: &mut Vec<u8>) {
        // VS_FIXEDFILEINFO 结构 (52 bytes)
        buffer.write_all(&0xFEEF04BDu32.to_le_bytes()).unwrap(); // dwSignature
        buffer.write_all(&0x00010000u32.to_le_bytes()).unwrap(); // dwStrucVersion
        
        // dwFileVersionMS, dwFileVersionLS
        let file_ms = ((self.file_version.0 as u32) << 16) | (self.file_version.1 as u32);
        let file_ls = ((self.file_version.2 as u32) << 16) | (self.file_version.3 as u32);
        buffer.write_all(&file_ms.to_le_bytes()).unwrap();
        buffer.write_all(&file_ls.to_le_bytes()).unwrap();
        
        // dwProductVersionMS, dwProductVersionLS
        let prod_ms = ((self.product_version.0 as u32) << 16) | (self.product_version.1 as u32);
        let prod_ls = ((self.product_version.2 as u32) << 16) | (self.product_version.3 as u32);
        buffer.write_all(&prod_ms.to_le_bytes()).unwrap();
        buffer.write_all(&prod_ls.to_le_bytes()).unwrap();
        
        buffer.write_all(&0x0000003Fu32.to_le_bytes()).unwrap(); // dwFileFlagsMask
        buffer.write_all(&0x00000000u32.to_le_bytes()).unwrap(); // dwFileFlags
        buffer.write_all(&0x00040004u32.to_le_bytes()).unwrap(); // dwFileOS (VOS_NT_WINDOWS32)
        buffer.write_all(&0x00000001u32.to_le_bytes()).unwrap(); // dwFileType (VFT_APP)
        buffer.write_all(&0x00000000u32.to_le_bytes()).unwrap(); // dwFileSubtype
        buffer.write_all(&0x00000000u32.to_le_bytes()).unwrap(); // dwFileDateMS
        buffer.write_all(&0x00000000u32.to_le_bytes()).unwrap(); // dwFileDateLS
    }
    
    fn write_string_file_info(&self, buffer: &mut Vec<u8>) {
        let start_pos = buffer.len();
        
        // 预留 wLength
        buffer.write_all(&0u16.to_le_bytes()).unwrap();
        
        // wValueLength = 0 (没有直接 value)
        buffer.write_all(&0u16.to_le_bytes()).unwrap();
        
        // wType = 1 (text)
        buffer.write_all(&1u16.to_le_bytes()).unwrap();
        
        // szKey = "StringFileInfo"
        write_wstring(buffer, "StringFileInfo");
        
        // 对齐
        pad_to_dword(buffer);
        
        // Children: StringTable
        self.write_string_table(buffer);
        
        // 对齐
        pad_to_dword(buffer);
        
        // 回填 wLength
        let total_length = buffer.len() - start_pos;
        let length_bytes = (total_length as u16).to_le_bytes();
        buffer[start_pos] = length_bytes[0];
        buffer[start_pos + 1] = length_bytes[1];
    }
    
    fn write_string_table(&self, buffer: &mut Vec<u8>) {
        let start_pos = buffer.len();
        
        // 预留 wLength
        buffer.write_all(&0u16.to_le_bytes()).unwrap();
        
        // wValueLength = 0
        buffer.write_all(&0u16.to_le_bytes()).unwrap();
        
        // wType = 1 (text)
        buffer.write_all(&1u16.to_le_bytes()).unwrap();
        
        // szKey = "040904B0" (英语 + Unicode)
        write_wstring(buffer, "040904B0");
        
        // 对齐
        pad_to_dword(buffer);
        
        // Children: String 结构
        for (key, value) in &self.strings {
            self.write_string(buffer, key, value);
            pad_to_dword(buffer);
        }
        
        // 回填 wLength
        let total_length = buffer.len() - start_pos;
        let length_bytes = (total_length as u16).to_le_bytes();
        buffer[start_pos] = length_bytes[0];
        buffer[start_pos + 1] = length_bytes[1];
    }
    
    fn write_string(&self, buffer: &mut Vec<u8>, key: &str, value: &str) {
        let start_pos = buffer.len();
        
        // 预留 wLength
        buffer.write_all(&0u16.to_le_bytes()).unwrap();
        
        // wValueLength = value 的字符数（不包括 null terminator）
        let value_len = value.encode_utf16().count();
        buffer.write_all(&(value_len as u16).to_le_bytes()).unwrap();
        
        // wType = 1 (text)
        buffer.write_all(&1u16.to_le_bytes()).unwrap();
        
        // szKey
        write_wstring(buffer, key);
        
        // 对齐
        pad_to_dword(buffer);
        
        // Value (不包括 null terminator，因为 wValueLength 不包括)
        for ch in value.encode_utf16() {
            buffer.write_all(&ch.to_le_bytes()).unwrap();
        }
        // 添加 null terminator
        buffer.write_all(&0u16.to_le_bytes()).unwrap();
        
        // 回填 wLength
        let total_length = buffer.len() - start_pos;
        let length_bytes = (total_length as u16).to_le_bytes();
        buffer[start_pos] = length_bytes[0];
        buffer[start_pos + 1] = length_bytes[1];
    }
    
    fn write_var_file_info(&self, buffer: &mut Vec<u8>) {
        let start_pos = buffer.len();
        
        // 预留 wLength
        buffer.write_all(&0u16.to_le_bytes()).unwrap();
        
        // wValueLength = 0
        buffer.write_all(&0u16.to_le_bytes()).unwrap();
        
        // wType = 1 (text)
        buffer.write_all(&1u16.to_le_bytes()).unwrap();
        
        // szKey = "VarFileInfo"
        write_wstring(buffer, "VarFileInfo");
        
        // 对齐
        pad_to_dword(buffer);
        
        // Children: Var
        self.write_var(buffer);
        
        // 对齐
        pad_to_dword(buffer);
        
        // 回填 wLength
        let total_length = buffer.len() - start_pos;
        let length_bytes = (total_length as u16).to_le_bytes();
        buffer[start_pos] = length_bytes[0];
        buffer[start_pos + 1] = length_bytes[1];
    }
    
    fn write_var(&self, buffer: &mut Vec<u8>) {
        let start_pos = buffer.len();
        
        // 预留 wLength
        buffer.write_all(&0u16.to_le_bytes()).unwrap();
        
        // wValueLength = 4 (一个 DWORD)
        buffer.write_all(&4u16.to_le_bytes()).unwrap();
        
        // wType = 0 (binary)
        buffer.write_all(&0u16.to_le_bytes()).unwrap();
        
        // szKey = "Translation"
        write_wstring(buffer, "Translation");
        
        // 对齐
        pad_to_dword(buffer);
        
        // Value: Language ID (0x0409 = 英语) + Code Page (0x04B0 = Unicode)
        buffer.write_all(&0x0409u16.to_le_bytes()).unwrap();
        buffer.write_all(&0x04B0u16.to_le_bytes()).unwrap();
        
        // 回填 wLength
        let total_length = buffer.len() - start_pos;
        let length_bytes = (total_length as u16).to_le_bytes();
        buffer[start_pos] = length_bytes[0];
        buffer[start_pos + 1] = length_bytes[1];
    }
}

fn write_wstring(buffer: &mut Vec<u8>, s: &str) {
    for ch in s.encode_utf16() {
        buffer.write_all(&ch.to_le_bytes()).unwrap();
    }
    // Null terminator
    buffer.write_all(&0u16.to_le_bytes()).unwrap();
}

fn pad_to_dword(buffer: &mut Vec<u8>) {
    while buffer.len() % 4 != 0 {
        buffer.push(0);
    }
}
