use anyhow::{Context, Result, ensure};
use std::{
    fs,
    path::{Path, PathBuf},
};

pub(super) struct StagedOutput {
    destination: PathBuf,
    work: PathBuf,
    stage: PathBuf,
    lock: PathBuf,
}

impl StagedOutput {
    pub(super) fn new(destination: &Path) -> Result<Self> {
        let name = destination
            .file_name()
            .context("output must name a directory")?;
        let parent = destination
            .parent()
            .filter(|p| !p.as_os_str().is_empty())
            .unwrap_or(Path::new("."));
        fs::create_dir_all(parent)?;
        let parent = parent.canonicalize()?;
        let destination = parent.join(name);
        if let Ok(meta) = fs::symlink_metadata(&destination) {
            ensure!(
                meta.is_dir() && !meta.file_type().is_symlink(),
                "output must be a real directory"
            );
        }
        let lock = parent.join(format!(".{}.export.lock", name.to_string_lossy()));
        let file = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&lock)
            .with_context(|| {
                format!(
                    "another export may be running; output lock: {}",
                    lock.display()
                )
            })?;
        drop(file);
        let work = parent.join(format!(
            ".{}.export-{}-{}",
            name.to_string_lossy(),
            std::process::id(),
            chrono::Utc::now().timestamp_nanos_opt().unwrap_or_default()
        ));
        if let Err(error) = fs::create_dir(&work) {
            let _ = fs::remove_file(&lock);
            return Err(error.into());
        }
        let output = Self {
            destination,
            stage: work.join("site"),
            work,
            lock,
        };
        fs::create_dir(&output.stage)?;
        Ok(output)
    }

    pub(super) fn path(&self) -> &Path {
        &self.stage
    }

    pub(super) fn publish(self) -> Result<()> {
        self.publish_with(|from, to| fs::rename(from, to))
    }

    fn publish_with(self, rename: impl Fn(&Path, &Path) -> std::io::Result<()>) -> Result<()> {
        for required in ["index.html", "404.html", "search-index.json", "robots.txt"] {
            ensure!(
                self.stage.join(required).is_file(),
                "missing generated file: {required}"
            );
        }
        let backup = self.work.join("previous");
        let had_output = self.destination.exists();
        if had_output {
            rename(&self.destination, &backup).context("could not preserve previous output")?;
        }
        if let Err(error) = rename(&self.stage, &self.destination) {
            if had_output && let Err(restore) = rename(&backup, &self.destination) {
                anyhow::bail!(
                    "publish failed: {error}; restore failed: {restore}; previous output retained at {}",
                    backup.display()
                );
            }
            return Err(error).context("could not publish output; previous output restored");
        }
        if had_output {
            fs::remove_dir_all(&backup).with_context(|| {
                format!(
                    "output published, but backup cleanup failed: {}",
                    backup.display()
                )
            })?;
        }
        Ok(())
    }
}

impl Drop for StagedOutput {
    fn drop(&mut self) {
        if !self.work.join("previous").exists() {
            let _ = fs::remove_dir_all(&self.work);
        }
        let _ = fs::remove_file(&self.lock);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> (PathBuf, PathBuf) {
        let root = std::env::temp_dir().join(format!(
            "export-output-{}-{}",
            std::process::id(),
            chrono::Utc::now().timestamp_nanos_opt().unwrap()
        ));
        let out = root.join("dist");
        fs::create_dir_all(&out).unwrap();
        fs::write(out.join("old.html"), "old").unwrap();
        (root, out)
    }

    fn populate(output: &StagedOutput) {
        for name in ["index.html", "404.html", "search-index.json", "robots.txt"] {
            fs::write(output.path().join(name), "new").unwrap();
        }
    }

    #[test]
    fn failed_generation_preserves_output_and_cleans_staging() {
        let (root, out) = fixture();
        let staged = StagedOutput::new(&out).unwrap();
        assert!(StagedOutput::new(&out).is_err());
        fs::write(staged.path().join("partial.html"), "partial").unwrap();
        assert!(staged.publish().is_err());
        assert_eq!(fs::read_to_string(out.join("old.html")).unwrap(), "old");
        assert_eq!(fs::read_dir(&root).unwrap().count(), 1);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn successful_publish_removes_stale_files() {
        let (root, out) = fixture();
        let staged = StagedOutput::new(&out).unwrap();
        populate(&staged);
        staged.publish().unwrap();
        assert!(!out.join("old.html").exists());
        assert_eq!(fs::read_to_string(out.join("index.html")).unwrap(), "new");
        assert_eq!(fs::read_dir(&root).unwrap().count(), 1);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn failed_publish_restores_previous_output() {
        let (root, out) = fixture();
        let staged = StagedOutput::new(&out).unwrap();
        populate(&staged);
        let result = staged.publish_with(|from, to| {
            if from.file_name().unwrap() == "site" {
                return Err(std::io::Error::other("injected publish failure"));
            }
            fs::rename(from, to)
        });
        assert!(result.is_err());
        assert_eq!(fs::read_to_string(out.join("old.html")).unwrap(), "old");
        assert_eq!(fs::read_dir(&root).unwrap().count(), 1);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn failed_restore_keeps_backup_for_manual_recovery() {
        let (root, out) = fixture();
        let staged = StagedOutput::new(&out).unwrap();
        let backup = staged.work.join("previous");
        populate(&staged);
        let result = staged.publish_with(|from, to| {
            if from == out {
                fs::rename(from, to)
            } else {
                Err(std::io::Error::other("injected rename failure"))
            }
        });
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("previous output retained")
        );
        assert_eq!(fs::read_to_string(backup.join("old.html")).unwrap(), "old");
        fs::remove_dir_all(root).unwrap();
    }
}
