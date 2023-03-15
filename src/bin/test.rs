
// main for testing
fn main() -> std::io::Result<()> {
    let app = sap_error_utils::apps::SapInboxApp::default();
    
    app.generate_comparison()?;

    Ok(())
}