use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use sha1::Sha1;

use common::{ID, EnumerableBlobStorage, BlobStorage};
use errors::{self, Error};

pub struct FileBlobStorage {
    path: PathBuf,
}

impl FileBlobStorage {
    pub fn open<P: AsRef<Path>>(path: P) -> FileBlobStorage {
        FileBlobStorage { path: path.as_ref().to_path_buf() }
    }

    fn filename(&self, id: &ID) -> PathBuf {
        let mut path = self.path.to_path_buf();
        path.push(::std::str::from_utf8(&id.bytes[..2]).unwrap());
        path.push(::std::str::from_utf8(&id.bytes[2..]).unwrap());
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
        let mut sha1 = Sha1::new();
        sha1.update(blob);
        let id = ID { bytes: sha1.digest().bytes() };
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
                        return Some(Err(Error::IoError("Error reading blobs directory", e)));
                    }
                };
                let name = match entry.file_name().into_string() {
                    Ok(v) => v,
                    Err(_) => {
                        return Some(Err(Error::CorruptedStore("First-level entry in blobs is invalid unicode")));
                    }
                };
                let slice = name.as_bytes();
                if slice.len() != 2 {
                    panic!("First-level entry has invalid length {}",
                           slice.len());
                }
                self.first_val.clone_from_slice(slice);
                self.second = Some(entry.path()
                    .read_dir()
                    .expect("Error reading first-level entry in blobs"));
            } else {
                return None;
            }
        }
        if let Some(entry) = self.second.as_mut().unwrap().next() {
            let entry =
                entry.expect("Error reading second-level entry in blobs");
            let mut id = [0u8; 20];
            id[..2].clone_from_slice(&self.first_val);
            let name = entry.file_name()
                .into_string()
                .expect("Second-level entry in blobs is invalid unicode");
            let slice = name.as_bytes();
            if slice.len() != 18 {
                panic!("Second-level entry has invalid length {}",
                       slice.len());
            }
            id[2..].clone_from_slice(slice);
            Some(Ok(ID { bytes: id }))
        } else {
            self.second = None;
            self.next()
        }
    }
}
