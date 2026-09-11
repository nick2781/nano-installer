#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod runtime;

fn main() -> anyhow::Result<()> {
    runtime::run()
}
