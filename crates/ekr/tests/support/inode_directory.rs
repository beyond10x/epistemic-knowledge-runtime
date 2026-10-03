//! Both the inode-reuse probe and its consumer must stage on the same filesystem.
pub fn root() -> std::path::PathBuf {
    std::env::var_os("EKR_INODE_TEST_TMPDIR")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(std::env::temp_dir)
}

pub fn directory() -> tempfile::TempDir {
    tempfile::tempdir_in(root()).expect("inode test scratch directory")
}
