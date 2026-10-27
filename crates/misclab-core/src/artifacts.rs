//! Persistent out-of-band node results. UUIDs are opaque locators, never paths.
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::{CoreError, CoreResult};

pub const MAX_ARTIFACT_BYTES: usize = 128 * 1024 * 1024;
pub const MAX_READ_BYTES: usize = 1024 * 1024;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ArtifactMetadata {
    pub id: Uuid,
    pub name: String,
    pub mime: String,
    pub size: u64,
    pub sha256: String,
}

pub struct ArtifactRepository {
    root: PathBuf,
}
impl ArtifactRepository {
    pub fn new(root: PathBuf) -> Self {
        Self { root }
    }
    fn path(&self, id: Uuid, ext: &str) -> PathBuf {
        self.root.join(format!("{id}.{ext}"))
    }

    pub fn put(&self, name: &str, mime: &str, bytes: &[u8]) -> CoreResult<ArtifactMetadata> {
        if bytes.len() > MAX_ARTIFACT_BYTES {
            return Err(CoreError::BudgetExceeded("单个产物不能超过 128 MiB".into()));
        }
        fs::create_dir_all(&self.root)?;
        let metadata = ArtifactMetadata {
            id: Uuid::new_v4(),
            name: name
                .rsplit(['/', '\\'])
                .next()
                .filter(|n| !n.is_empty())
                .unwrap_or("output.bin")
                .chars()
                .filter(|c| !c.is_control())
                .take(200)
                .collect(),
            mime: mime.into(),
            size: bytes.len() as u64,
            sha256: hex::encode(Sha256::digest(bytes)),
        };
        let path = self.path(metadata.id, "bin");
        let result = (|| -> CoreResult<()> {
            let mut data = OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&path)?;
            data.write_all(bytes)?;
            let mut meta = OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(self.path(metadata.id, "json"))?;
            meta.write_all(
                &serde_json::to_vec(&metadata).map_err(|e| CoreError::Other(e.to_string()))?,
            )?;
            Ok(())
        })();
        if let Err(e) = result {
            let _ = fs::remove_file(path);
            let _ = fs::remove_file(self.path(metadata.id, "json"));
            return Err(e);
        }
        Ok(metadata)
    }

    pub fn metadata(&self, id: Uuid) -> CoreResult<ArtifactMetadata> {
        let bytes = fs::read(self.path(id, "json"))?;
        let metadata: ArtifactMetadata =
            serde_json::from_slice(&bytes).map_err(|e| CoreError::Parse(e.to_string()))?;
        if metadata.id != id || metadata.size != fs::metadata(self.path(id, "bin"))?.len() {
            return Err(CoreError::Parse("产物元数据与文件不一致".into()));
        }
        Ok(metadata)
    }

    pub fn read(&self, id: Uuid, offset: u64, length: usize) -> CoreResult<Vec<u8>> {
        if length > MAX_READ_BYTES {
            return Err(CoreError::BudgetExceeded("单次读取不能超过 1 MiB".into()));
        }
        let metadata = self.metadata(id)?;
        if offset > metadata.size {
            return Err(CoreError::Parse("产物读取偏移超出范围".into()));
        }
        let mut file = File::open(self.path(id, "bin"))?;
        file.seek(SeekFrom::Start(offset))?;
        let mut out = Vec::new();
        file.take(length as u64).read_to_end(&mut out)?;
        Ok(out)
    }

    pub fn read_all(&self, id: Uuid, limit: usize) -> CoreResult<Vec<u8>> {
        let metadata = self.metadata(id)?;
        if metadata.size > limit.min(MAX_ARTIFACT_BYTES) as u64 {
            return Err(CoreError::BudgetExceeded(
                "产物超过读取节点的字节上限".into(),
            ));
        }
        Ok(fs::read(self.path(id, "bin"))?)
    }

    pub fn export(&self, id: Uuid, destination: &Path) -> CoreResult<()> {
        self.metadata(id)?;
        let mut src = File::open(self.path(id, "bin"))?;
        // An explicit export must not silently overwrite an existing file.
        let mut dst = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(destination)?;
        std::io::copy(&mut src, &mut dst)?;
        Ok(())
    }
}

static REPOSITORY: OnceLock<ArtifactRepository> = OnceLock::new();
pub fn configure(root: PathBuf) {
    let _ = REPOSITORY.set(ArtifactRepository::new(root));
}
pub fn repository() -> &'static ArtifactRepository {
    REPOSITORY.get_or_init(|| {
        ArtifactRepository::new(std::env::temp_dir().join("LovelyMiscLab-artifacts"))
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn persists_binary_metadata_ranges_and_explicit_export() {
        let root = std::env::temp_dir().join(format!("misclab-artifact-test-{}", Uuid::new_v4()));
        let repo = ArtifactRepository::new(root.clone());
        let data: Vec<u8> = (0..=255).collect();
        let a = repo
            .put("../载体.bin", "application/octet-stream", &data)
            .unwrap();
        assert_eq!(a.name, "载体.bin");
        assert_eq!(
            ArtifactRepository::new(root.clone())
                .metadata(a.id)
                .unwrap()
                .sha256,
            hex::encode(Sha256::digest(&data))
        );
        assert_eq!(repo.read(a.id, 250, 30).unwrap(), data[250..]);
        assert!(repo.read(a.id, 257, 1).is_err());
        assert!(repo.read(a.id, 0, MAX_READ_BYTES + 1).is_err());
        assert!(repo.read_all(a.id, 255).is_err());
        assert!(repo.metadata(Uuid::new_v4()).is_err());
        let dest = root.join("export.bin");
        repo.export(a.id, &dest).unwrap();
        assert_eq!(fs::read(&dest).unwrap(), data);
        assert!(repo.export(a.id, &dest).is_err());
        for p in [dest, repo.path(a.id, "bin"), repo.path(a.id, "json")] {
            fs::remove_file(p).unwrap();
        }
        fs::remove_dir(root).unwrap();
    }
}
