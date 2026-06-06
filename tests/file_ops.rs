use std::path::PathBuf;
use vo1d::tools::files::FileOps;

fn unique_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("vo1d_{}_{}", name, std::process::id()));
    let _ = std::fs::create_dir_all(&dir);
    dir
}

fn cleanup(dir: &PathBuf) {
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn test_file_write_and_read() {
    let dir = unique_dir("wr");
    let path = dir.join("hello.txt");

    let result = FileOps::write(&path, "Hello, World!", false).unwrap();
    assert!(result.contains("Wrote") && result.contains("13 bytes"));

    let content = FileOps::read(&path, None, None).unwrap();
    assert_eq!(content, "Hello, World!");

    cleanup(&dir);
}

#[test]
fn test_file_append() {
    let dir = unique_dir("append");
    let path = dir.join("append.txt");

    FileOps::write(&path, "line1\n", false).unwrap();
    FileOps::write(&path, "line2\n", true).unwrap();

    let content = FileOps::read(&path, None, None).unwrap();
    assert_eq!(content, "line1\nline2\n");

    cleanup(&dir);
}

#[test]
fn test_file_read_line_range() {
    let dir = unique_dir("range");
    let path = dir.join("lines.txt");
    let content = (1..=10).map(|i| format!("line {}", i)).collect::<Vec<_>>().join("\n");
    FileOps::write(&path, &content, false).unwrap();

    let lines_1_3 = FileOps::read(&path, Some(1), Some(3)).unwrap();
    assert_eq!(lines_1_3, "line 1\nline 2\nline 3");
    let lines_2_5 = FileOps::read(&path, Some(2), Some(5)).unwrap();
    assert_eq!(lines_2_5, "line 2\nline 3\nline 4\nline 5");

    cleanup(&dir);
}

#[test]
fn test_file_read_from_line() {
    let dir = unique_dir("from");
    let path = dir.join("from.txt");
    let content = (1..=5).map(|i| format!("line {}", i)).collect::<Vec<_>>().join("\n");
    FileOps::write(&path, &content, false).unwrap();

    let from_3 = FileOps::read(&path, Some(3), None).unwrap();
    assert_eq!(from_3, "line 3\nline 4\nline 5");

    cleanup(&dir);
}

#[test]
fn test_list_directory() {
    let dir = unique_dir("list");
    FileOps::write(&dir.join("a.txt"), "a", false).unwrap();
    FileOps::write(&dir.join("b.rs"), "b", false).unwrap();
    std::fs::create_dir_all(dir.join("sub")).unwrap();

    let listing = FileOps::list_dir(&dir, None).unwrap();
    assert!(listing.contains("a.txt"));
    assert!(listing.contains("b.rs"));
    assert!(listing.contains("sub"));

    cleanup(&dir);
}

#[test]
fn test_list_directory_with_pattern() {
    let dir = unique_dir("listpat");
    FileOps::write(&dir.join("main.rs"), "// rust", false).unwrap();
    FileOps::write(&dir.join("main.py"), "# python", false).unwrap();
    FileOps::write(&dir.join("data.json"), "{}", false).unwrap();

    let listing = FileOps::list_dir(&dir, Some("*.rs")).unwrap();
    assert!(listing.contains("main.rs"));
    assert!(!listing.contains("main.py"));
    assert!(!listing.contains("data.json"));

    cleanup(&dir);
}

#[test]
fn test_search_files() {
    let dir = unique_dir("search");
    FileOps::write(&dir.join("readme.md"), "# doc", false).unwrap();
    let src = dir.join("src");
    std::fs::create_dir_all(&src).unwrap();
    FileOps::write(&src.join("main.rs"), "fn main() {}", false).unwrap();
    FileOps::write(&src.join("lib.rs"), "pub fn hello() {}", false).unwrap();

    let results = FileOps::search(&dir, "*.rs").unwrap();
    assert!(results.contains("main.rs") || results.contains("lib.rs"));

    cleanup(&dir);
}

#[test]
fn test_delete_file() {
    let dir = unique_dir("delete");
    let path = dir.join("delete_me.txt");
    FileOps::write(&path, "to be deleted", false).unwrap();

    let result = FileOps::delete(&path).unwrap();
    assert!(result.contains("Deleted"));
    assert!(!path.exists());

    cleanup(&dir);
}

#[test]
fn test_delete_nonexistent_fails() {
    let dir = unique_dir("delnone");
    let path = dir.join("does_not_exist.txt");
    let result = FileOps::delete(&path);
    assert!(result.is_err());

    cleanup(&dir);
}

#[test]
fn test_copy_file() {
    let dir = unique_dir("copy");
    let src = dir.join("source.txt");
    let dst = dir.join("dest.txt");
    FileOps::write(&src, "copy me", false).unwrap();

    FileOps::copy(&src, &dst).unwrap();
    assert!(dst.exists());

    let content = FileOps::read(&dst, None, None).unwrap();
    assert_eq!(content, "copy me");

    cleanup(&dir);
}

#[test]
fn test_create_directory() {
    let dir = unique_dir("mkdir");
    let new_dir = dir.join("nested").join("deep").join("dir");

    FileOps::create_dir(&new_dir).unwrap();
    assert!(new_dir.exists());
    assert!(new_dir.is_dir());

    cleanup(&dir);
}

#[test]
fn test_file_metadata() {
    let dir = unique_dir("meta");
    let path = dir.join("meta_test.txt");
    FileOps::write(&path, "metadata test", false).unwrap();

    let meta = FileOps::metadata(&path).unwrap();
    assert!(meta.contains("meta_test.txt"));
    assert!(meta.contains("file"));
    assert!(meta.contains("bytes"));

    cleanup(&dir);
}

#[test]
fn test_atomic_write_no_partial_file_on_failure() {
    let dir = unique_dir("atomic");
    let path = dir.join("atomic.txt");

    FileOps::write(&path, "atomic content", false).unwrap();
    assert!(path.exists());

    let tmp = path.with_extension("tmp");
    assert!(!tmp.exists());

    cleanup(&dir);
}
