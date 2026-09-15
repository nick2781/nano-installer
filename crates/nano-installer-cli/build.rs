include!("../../build-support/windows_tool_icon.rs");

const BRAND_PNG: &[u8] = include_bytes!("../../assets/nano-technology-cli.png");
const BRAND_PNG_PATH: &str = "../../assets/nano-technology-cli.png";

fn main() {
    embed_tool_icon(
        BRAND_PNG,
        BRAND_PNG_PATH,
        "nano-installer-native.ico",
        "nano-installer command-line builder",
        "nano-installer-native-x64.exe",
    );
}
