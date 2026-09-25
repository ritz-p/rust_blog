use anyhow::{Context, Result, ensure};
use chrono::{DateTime, Timelike, Utc};
use std::{path::Path, process::Command};

/// Use content history, not checkout mtime, for clean tracked Markdown.
pub fn updated_at(path: &Path) -> Result<DateTime<Utc>> {
    let path = path.canonicalize()?;
    let modified = || -> Result<DateTime<Utc>> {
        let time: DateTime<Utc> = path.metadata()?.modified()?.into();
        Ok(time.with_nanosecond(0).unwrap_or(time))
    };
    let Some(root) = path
        .ancestors()
        .skip(1)
        .find(|dir| dir.join(".git").exists())
    else {
        return modified();
    };
    let relative = path.strip_prefix(root)?;
    let git = |args: &[&str], with_path: bool| -> Result<std::process::Output> {
        let mut command = Command::new("git");
        command
            .arg("-c")
            .arg(format!("safe.directory={}", root.display()))
            .arg("--literal-pathspecs")
            .arg("-C")
            .arg(root)
            .args(args);
        if with_path {
            command.arg("--").arg(relative);
        }
        command.output().context("read Markdown Git history")
    };
    let tracked = git(&["ls-files", "--error-unmatch"], true)?;
    if !tracked.status.success() {
        return modified();
    }
    let shallow = git(&["rev-parse", "--is-shallow-repository"], false)?;
    ensure!(
        shallow.status.success(),
        "cannot inspect Git history: {}",
        String::from_utf8_lossy(&shallow.stderr)
    );
    ensure!(
        String::from_utf8_lossy(&shallow.stdout).trim() == "false",
        "file timestamps require full Git history (checkout fetch-depth: 0)"
    );
    let diff = git(&["diff", "--quiet", "HEAD"], true)?;
    if diff.status.code() == Some(1) {
        return modified();
    }
    ensure!(
        diff.status.success(),
        "cannot compare Markdown to HEAD: {}",
        String::from_utf8_lossy(&diff.stderr)
    );
    let log = git(&["log", "--follow", "-1", "--format=%cI"], true)?;
    ensure!(
        log.status.success(),
        "cannot read Markdown history: {}",
        String::from_utf8_lossy(&log.stderr)
    );
    let text = String::from_utf8(log.stdout)?;
    if text.trim().is_empty() {
        return modified();
    }
    Ok(DateTime::parse_from_rfc3339(text.trim())?.with_timezone(&Utc))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn committed_content_ignores_checkout_time_and_unrelated_commits() {
        let root = std::env::temp_dir().join(format!(
            "file-time-{}-{}",
            std::process::id(),
            Utc::now().timestamp_nanos_opt().unwrap()
        ));
        fs::create_dir_all(&root).unwrap();
        let git = |args: &[&str], date: &str| {
            let output = Command::new("git")
                .arg("-C")
                .arg(&root)
                .args(["-c", "user.name=Test", "-c", "user.email=test@example.com"])
                .env("GIT_AUTHOR_DATE", date)
                .env("GIT_COMMITTER_DATE", date)
                .args(args)
                .output()
                .unwrap();
            assert!(
                output.status.success(),
                "{}",
                String::from_utf8_lossy(&output.stderr)
            );
        };
        let first = "2020-01-01T00:00:00Z";
        let second = "2021-01-01T00:00:00Z";
        git(&["init"], first);
        let path = root.join("article.md");
        fs::write(&path, "first body").unwrap();
        git(&["add", "."], first);
        git(&["commit", "-m", "first"], first);
        let expected = DateTime::parse_from_rfc3339(first)
            .unwrap()
            .with_timezone(&Utc);
        assert_eq!(updated_at(&path).unwrap(), expected);
        fs::write(&path, "first body").unwrap(); // same bytes, new mtime
        fs::write(root.join("other.md"), "unrelated").unwrap();
        git(&["add", "."], second);
        git(&["commit", "-m", "unrelated"], second);
        assert_eq!(updated_at(&path).unwrap(), expected);
        fs::write(&path, "changed body").unwrap();
        assert!(updated_at(&path).unwrap() > expected);
        git(&["add", "."], second);
        git(&["commit", "-m", "changed"], second);
        assert_eq!(
            updated_at(&path).unwrap(),
            DateTime::parse_from_rfc3339(second)
                .unwrap()
                .with_timezone(&Utc)
        );
        fs::remove_dir_all(root).unwrap();
    }
}
