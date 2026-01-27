//! Integration tests for RemoteFileSystem
//!
//! These tests spawn the Python agent locally and use the RemoteFileSystem
//! through AgentChannel to test the full integration stack.
//!
//! These tests use the production code paths:
//! - spawn_local_agent() for agent creation
//! - AgentChannel for communication
//! - RemoteFileSystem for file operations

use fresh::model::filesystem::FileSystem;
use fresh::services::remote::{spawn_local_agent, RemoteFileSystem};

/// Creates a RemoteFileSystem using production code
fn create_test_filesystem() -> Option<(RemoteFileSystem, tempfile::TempDir, tokio::runtime::Runtime)>
{
    let temp_dir = tempfile::tempdir().ok()?;
    let rt = tokio::runtime::Runtime::new().ok()?;

    let channel = rt.block_on(spawn_local_agent()).ok()?;
    let fs = RemoteFileSystem::new(channel, "test@localhost".to_string());

    Some((fs, temp_dir, rt))
}

#[test]
fn test_read_file_content() {
    let Some((fs, temp_dir, _rt)) = create_test_filesystem() else {
        eprintln!("Skipping test: could not create test filesystem");
        return;
    };

    let test_path = temp_dir.path().join("test.txt");
    let test_content = b"Hello, this is test content!\nLine 2\nLine 3";

    // Write file using std::fs (directly to the temp dir)
    std::fs::write(&test_path, test_content).unwrap();

    // Read via RemoteFileSystem
    let read_content = fs.read_file(&test_path).unwrap();

    assert_eq!(
        read_content, test_content,
        "File content should match what was written"
    );
}

#[test]
fn test_write_and_read_roundtrip() {
    let Some((fs, temp_dir, _rt)) = create_test_filesystem() else {
        eprintln!("Skipping test: could not create test filesystem");
        return;
    };

    let test_path = temp_dir.path().join("write_test.txt");
    let test_content = b"Content written via RemoteFileSystem";

    // Write via RemoteFileSystem
    fs.write_file(&test_path, test_content).unwrap();

    // Read back via RemoteFileSystem
    let read_content = fs.read_file(&test_path).unwrap();

    assert_eq!(
        read_content, test_content,
        "Read content should match written content"
    );

    // Also verify via std::fs
    let direct_read = std::fs::read(&test_path).unwrap();
    assert_eq!(
        direct_read, test_content,
        "Direct file read should match written content"
    );
}

#[test]
fn test_read_large_file() {
    let Some((fs, temp_dir, _rt)) = create_test_filesystem() else {
        eprintln!("Skipping test: could not create test filesystem");
        return;
    };

    let test_path = temp_dir.path().join("large.bin");

    // Create a file larger than the chunk size (65536 bytes)
    let test_content: Vec<u8> = (0..100_000).map(|i| (i % 256) as u8).collect();

    std::fs::write(&test_path, &test_content).unwrap();

    // Read via RemoteFileSystem (should handle multiple streaming chunks)
    let read_content = fs.read_file(&test_path).unwrap();

    assert_eq!(
        read_content.len(),
        test_content.len(),
        "File sizes should match"
    );
    assert_eq!(
        read_content, test_content,
        "Large file content should match"
    );
}

#[test]
fn test_is_dir() {
    let Some((fs, temp_dir, _rt)) = create_test_filesystem() else {
        eprintln!("Skipping test: could not create test filesystem");
        return;
    };

    let dir_path = temp_dir.path().join("subdir");
    let file_path = temp_dir.path().join("file.txt");

    std::fs::create_dir(&dir_path).unwrap();
    std::fs::write(&file_path, b"content").unwrap();

    assert!(fs.is_dir(&dir_path).unwrap(), "Should detect directory");
    assert!(!fs.is_dir(&file_path).unwrap(), "File should not be a dir");
}

#[test]
fn test_read_dir() {
    let Some((fs, temp_dir, _rt)) = create_test_filesystem() else {
        eprintln!("Skipping test: could not create test filesystem");
        return;
    };

    std::fs::write(temp_dir.path().join("file1.txt"), b"1").unwrap();
    std::fs::write(temp_dir.path().join("file2.txt"), b"2").unwrap();
    std::fs::create_dir(temp_dir.path().join("subdir")).unwrap();

    let entries = fs.read_dir(temp_dir.path()).unwrap();
    let names: Vec<_> = entries.iter().map(|e| e.name.as_str()).collect();

    assert!(names.contains(&"file1.txt"), "Should contain file1.txt");
    assert!(names.contains(&"file2.txt"), "Should contain file2.txt");
    assert!(names.contains(&"subdir"), "Should contain subdir");
}

#[test]
fn test_remote_connection_info() {
    let Some((fs, _temp_dir, _rt)) = create_test_filesystem() else {
        eprintln!("Skipping test: could not create test filesystem");
        return;
    };

    assert_eq!(
        fs.remote_connection_info(),
        Some("test@localhost"),
        "Should return connection string"
    );
}

#[test]
fn test_metadata() {
    let Some((fs, temp_dir, _rt)) = create_test_filesystem() else {
        eprintln!("Skipping test: could not create test filesystem");
        return;
    };

    let test_path = temp_dir.path().join("meta_test.txt");
    let content = b"test content for metadata";
    std::fs::write(&test_path, content).unwrap();

    let meta = fs.metadata(&test_path).unwrap();
    assert_eq!(
        meta.size,
        content.len() as u64,
        "Size should match content length"
    );
}
