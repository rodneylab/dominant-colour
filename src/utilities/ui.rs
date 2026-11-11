use arboard::Clipboard;

pub fn copy_text_to_clipboard(text: &str, description: Option<&str>) {
    let description = description.unwrap_or("text");
    if let Ok(mut clipboard) = Clipboard::new()
        && clipboard.set_text(text).is_ok()
    {
        log::info!("{description} copied to clipboard.");
    } else {
        log::error!("Failed to copy {description} to clipboard");
    }
}
