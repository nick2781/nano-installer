include!("../../build-support/windows_tool_icon.rs");

const BRAND_PNG: &[u8] = include_bytes!("../../assets/nano-technology.png");
const BRAND_PNG_PATH: &str = "../../assets/nano-technology.png";

fn main() {
    embed_tool_icon(
        BRAND_PNG,
        BRAND_PNG_PATH,
        "nano-installer-gui.ico",
        "nano-installer visual builder",
        "nano-installer-gui-x64.exe",
    );
}
