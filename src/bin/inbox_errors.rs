// hide terminal window, if not a debug build and terminal feature is not enabled
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

// TODO: cli
fn main() -> eframe::Result<()> {
    sap_error_utils::apps::SapInboxApp::run()
}
