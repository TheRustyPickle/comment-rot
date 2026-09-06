use std::fs;
use std::path::Path;
use tempfile::TempDir;

pub struct Project {
    dir: TempDir,
}

impl Project {
    pub fn new() -> Self {
        let dir = TempDir::new().expect("create temp project dir");
        Project { dir }
    }

    pub fn path(&self) -> &Path {
        self.dir.path()
    }

    /// Writes (or overwrites) a file at `rel`, creating parent directories
    /// as needed.
    pub fn write(&self, rel: &str, contents: &str) {
        let full = self.dir.path().join(rel);
        if let Some(parent) = full.parent() {
            fs::create_dir_all(parent).expect("create parent dir");
        }
        fs::write(full, contents).expect("write fixture file");
    }
}
