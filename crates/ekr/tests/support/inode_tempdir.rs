//! Both the inode-reuse guard and the replacement regression must use the same filesystem.

pub fn root() -> std::path::PathBuf {
    std::env::var_os("EKR_INODE_TEST_TMPDIR")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(std::env::temp_dir)
}

pub fn temporary() -> tempfile::TempDir {
    tempfile::tempdir_in(root()).expect("create inode-reuse fixture directory")
}
