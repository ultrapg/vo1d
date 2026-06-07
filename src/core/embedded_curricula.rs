/// Embedded curriculum JSON files bundled into the binary.
/// Used as fallback when curriculum files aren't found on disk.

pub const BUILTIN_NAMES: &[&str] = &[
    "00_hello_world",
    "01_file_ops",
    "02_directory_ops",
    "03_search_nav",
    "04_shell_basics",
    "05_web_basics",
    "06_rust_fix",
    "07_project_setup",
];

/// Look up an embedded curriculum by name (e.g. "00_hello_world").
pub fn get(name: &str) -> Option<&'static str> {
    match name {
        "00_hello_world" => Some(include_str!("../../curriculum/00_hello_world.json")),
        "01_file_ops" => Some(include_str!("../../curriculum/01_file_ops.json")),
        "02_directory_ops" => Some(include_str!("../../curriculum/02_directory_ops.json")),
        "03_search_nav" => Some(include_str!("../../curriculum/03_search_nav.json")),
        "04_shell_basics" => Some(include_str!("../../curriculum/04_shell_basics.json")),
        "05_web_basics" => Some(include_str!("../../curriculum/05_web_basics.json")),
        "06_rust_fix" => Some(include_str!("../../curriculum/06_rust_fix.json")),
        "07_project_setup" => Some(include_str!("../../curriculum/07_project_setup.json")),
        _ => None,
    }
}

/// List all embedded curriculum names.
pub fn list() -> &'static [&'static str] {
    BUILTIN_NAMES
}
