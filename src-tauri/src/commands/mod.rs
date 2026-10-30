pub mod files;
pub mod index;
pub mod search;
pub mod settings;
pub mod system;

pub fn now_ms() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0)
}
