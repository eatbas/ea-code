use std::path::{Path, PathBuf};

pub(super) struct TestWorkspace {
    path: PathBuf,
}

impl TestWorkspace {
    pub fn new() -> Self {
        let path = std::env::temp_dir().join(format!("maestro-test-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&path).expect("temporary workspace should be created");
        Self { path }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for TestWorkspace {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.path);
    }
}
