use super::Preview;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    fs,
    path::Path,
    time::{Duration, SystemTime},
};
use url::Url;

const LIFETIME: Duration = Duration::from_secs(24 * 60 * 60);

#[derive(Serialize, Deserialize)]
struct Entry {
    fetched: SystemTime,
    preview: Preview,
}

pub(super) async fn lookup<F, Fut>(directory: &Path, url: Url, fetch: F) -> Option<Preview>
where
    F: FnOnce() -> Fut,
    Fut: Future<Output = Option<Preview>>,
{
    let path = directory.join(format!("{:x}.json", Sha256::digest(url.as_str())));
    let cached: Option<Entry> = fs::File::open(&path)
        .ok()
        .and_then(|f| serde_json::from_reader(f).ok());
    if let Some(entry) = &cached
        && entry.fetched.elapsed().is_ok_and(|age| age < LIFETIME)
    {
        return Some(entry.preview.clone());
    }
    let Some(preview) = fetch().await else {
        return cached.map(|entry| entry.preview);
    };
    let entry = Entry {
        fetched: SystemTime::now(),
        preview,
    };
    let save = || -> anyhow::Result<()> {
        fs::create_dir_all(directory)?;
        let mut temp = tempfile::NamedTempFile::new_in(directory)?;
        serde_json::to_writer(&mut temp, &entry)?;
        temp.persist(&path)?;
        Ok(())
    };
    if let Err(error) = save() {
        eprintln!("link card cache {}: {error}", path.display());
    }
    Some(entry.preview)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[rocket::async_test]
    async fn disk_hit_skips_fetch_and_expired_entry_survives_failure() {
        let dir = tempfile::tempdir().unwrap();
        let url = Url::parse("https://example.org/").unwrap();
        let lookup = |url| {
            lookup(dir.path(), url, || async {
                Some(Preview {
                    title: Some("cached".into()),
                    description: None,
                    image: None,
                })
            })
        };
        lookup(url.clone()).await.unwrap();
        let hit = super::lookup(dir.path(), url.clone(), || async {
            panic!("fresh entry fetched")
        })
        .await
        .unwrap();
        assert_eq!(hit.title.as_deref(), Some("cached"));
        let path = fs::read_dir(dir.path())
            .unwrap()
            .next()
            .unwrap()
            .unwrap()
            .path();
        let entry = Entry {
            fetched: SystemTime::UNIX_EPOCH,
            preview: hit,
        };
        fs::write(path, serde_json::to_vec(&entry).unwrap()).unwrap();
        assert!(
            super::lookup(dir.path(), url, || async { None })
                .await
                .is_some()
        );
    }
}
