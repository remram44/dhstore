//! Blob storage implementation that stores blobs as files.
//!
//! This stores each blob in a separate file, and lists them by listing
//! directory contents. It is very similar to Git's loose objects directory.

use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use common::{ID, EnumerableBlobStorage, BlobStorage};
use errors::{self, Error};
use hash::Hasher;

/// Filesystem-based blob storage implementation.
///
/// This stores each blob in a separate file, and lists them by listing
/// directory contents. It is very similar to Git's loose object directory.
pub struct FileBlobStorage {
    path: PathBuf,
}

impl FileBlobStorage {
    /// Opens the blob storage from a path.
    pub fn open<P: AsRef<Path>>(path: P) -> FileBlobStorage {
        FileBlobStorage { path: path.as_ref().to_path_buf() }
    }

    /// Builds the path to an object from its ID.
    fn filename(&self, id: &ID) -> PathBuf {
        let mut path = self.path.to_path_buf();
        let hex = id.hex();
        path.push(&hex[..2]);
        path.push(&hex[2..]);
        path
    }
}

impl BlobStorage for FileBlobStorage {
    fn get_blob(&self, id: &ID) -> errors::Result<Option<Box<[u8]>>> {
        let path = self.filename(id);
        if path.exists() {
            let mut fp = File::open(path)
                .map_err(|e| ("Can't open blob file", e))?;
            let mut buf = Vec::new();
            fp.read_to_end(&mut buf)
                .map_err(|e| ("Error reading blob file", e))?;
            Ok(Some(buf.into_boxed_slice()))
        } else {
            Ok(None)
        }
    }

    fn add_blob(&mut self, blob: &[u8]) -> errors::Result<ID> {
        let mut hasher = Hasher::new();
        hasher.write_all(blob).unwrap();
        let id = hasher.result();
        self.add_known_blob(&id, blob)?;
        Ok(id)
    }

    fn add_known_blob(&mut self, id: &ID, blob: &[u8]) -> errors::Result<()> {
        let path = self.filename(id);
        if !path.exists() {
            {
                let parent = path.parent().unwrap();
                if !parent.exists() {
                    fs::create_dir(parent)
                        .map_err(|e| ("Couldn't create blob directory", e))?;
                }
            }
            let mut fp = OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(path)
                .map_err(|e| ("Can't open new blob file", e))?;
            fp.write_all(blob).map_err(|e| ("Error writing blob file", e))?;
        }
        Ok(())
    }

    fn delete_blob(&mut self, id: &ID) -> errors::Result<()> {
        let path = self.filename(id);
        if path.exists() {
            fs::remove_file(path)
                .map_err(|e| ("Couldn't remove blob file", e))?;
        }
        Ok(())
    }

    fn verify(&mut self) -> errors::Result<()> {
        for blob in self.list_blobs()? {
            match blob {
                Err(e) => error!("Error listing blobs: {}", e),
                Ok(id) => {
                    let mut hasher = Hasher::new();
                    match self.get_blob(&id) {
                        Err(e) => error!("Error getting blob: {}", e),
                        Ok(None) => error!("Error gettting blob"),
                        Ok(Some(blob)) => {
                            hasher.write_all(&blob).unwrap();
                            if id != hasher.result() {
                                warn!("Blob has the wrong hash: {:?}",
                                      self.filename(&id));
                            } else {
                                info!("Checked {}", id);
                            }
                        }
                    }
                }
            }
        }
        Ok(())
    }
}

impl EnumerableBlobStorage for FileBlobStorage {
    type Iter = FileBlobIterator;

    fn list_blobs(&self) -> errors::Result<FileBlobIterator> {
        let first = self.path
            .read_dir()
            .map_err(|e| ("Blobs directory doesn't exist", e))?;
        Ok(FileBlobIterator {
            first: first,
            first_val: [0u8; 2],
            second: None,
        })
    }
}

/// Iterator on blobs returned by `FileBlobStorage::list_blobs()`.
///
/// Simply uses `Path::read_dir()` to list directory contents and parse the
/// paths back into `ID`s.
///
/// Note that filesystem operations can fail. If during iteration, one element
/// is `Err(...)`, you should abort iteration.
pub struct FileBlobIterator {
    first: fs::ReadDir,
    first_val: [u8; 2],
    second: Option<fs::ReadDir>,
}

impl Iterator for FileBlobIterator {
    type Item = errors::Result<ID>;

    fn next(&mut self) -> Option<errors::Result<ID>> {
        if self.second.is_none() {
            if let Some(entry) = self.first.next() {
                let entry = match entry {
                    Ok(v) => v,
                    Err(e) => {
                        return Some(Err(Error::IoError(
                            "Error reading blobs directory",
                            e)));
                    }
                };
                let name = match entry.file_name().into_string() {
                    Ok(v) => v,
                    Err(_) => {
                        return Some(Err(Error::CorruptedStore(
                            "First-level entry in blobs is invalid unicode")));
                    }
                };
                let slice = name.as_bytes();
                if slice.len() != 2 {
                    return Some(Err(Error::CorruptedStore(
                        "First-level entry has invalid length")));
                }
                self.first_val.clone_from_slice(slice);
                match entry.path().read_dir() {
                    Err(e) => {
                        return Some(Err(Error::IoError(
                            "Error reading subdirectory in blobs",
                            e)));
                    }
                    Ok(entry) => self.second = Some(entry),
                }
            } else {
                return None;
            }
        }
        if let Some(entry) = self.second.as_mut().unwrap().next() {
            if let Err(e) = entry {
                return Some(Err(Error::IoError(
                    "Error reading subdirectory in blobs",
                    e)));
            }
            let entry = entry.unwrap();
            let mut id = [0u8; 64];
            id[..2].clone_from_slice(&self.first_val);
            let name = entry.file_name()
                .into_string();
            if let Err(_) = name {
                return Some(Err(Error::CorruptedStore(
                    "Second-level entry in blobs is invalid unicode")));
            }
            let name = name.unwrap();
            let slice = name.as_bytes();
            if slice.len() != 62 {
                return Some(Err(Error::CorruptedStore(
                    "Second-level entry has invalid length")));
            }
            id[2..].clone_from_slice(slice);
            Some(ID::from_hex(&id)
                 .ok_or(Error::CorruptedStore("Path is not a valid ID")))
        } else {
            self.second = None;
            self.next()
        }
    }
}
