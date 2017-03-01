use std::env::temp_dir;
use std::fmt;
use std::fs::{self, File, OpenOptions};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};

use common::{ID, EnumerableBlobStorage, BlobStorage};
use errors::{self, Error};
use hash::Hasher;

pub struct FileBlobStorage {
    path: PathBuf,
}

impl FileBlobStorage {
    pub fn open<P: AsRef<Path>>(path: P) -> FileBlobStorage {
        FileBlobStorage { path: path.as_ref().to_path_buf() }
    }

    fn filename(&self, id: &ID) -> PathBuf {
        let mut path = self.path.to_path_buf();
        let hex = id.hex();
        path.push(&hex[..2]);
        path.push(&hex[2..]);
        path
    }
}

struct TmpFile {
    path: Option<PathBuf>,
}

impl TmpFile {
    fn new() -> TmpFile {
        // TODO: Use random suffix
        TmpFile { path: Some(temp_dir().join("dhstore_tmp")) }
    }

    fn open(&self, opts: &OpenOptions) -> io::Result<File> {
        opts.open(self.path.as_ref().unwrap())
    }

    fn rename<P: AsRef<Path>>(mut self, destination: P) -> io::Result<()> {
        let res = fs::rename(self.path.as_ref().unwrap(), destination);
        self.path = None;
        res
    }
}

impl Drop for TmpFile {
    fn drop(&mut self) {
        if let Some(ref path) = self.path {
            if let Err(e) = fs::remove_file(path) {
                warn!("Couldn't remove {:?}: {}", path, e);
            }
        }
    }
}

impl fmt::Debug for TmpFile {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        self.path.fmt(f)
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
        hasher.update(blob);
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

    fn copy_blob<R: Read>(&mut self, mut blob: R) -> errors::Result<ID> {
        let tmpfile = TmpFile::new();
        let id = {
            let mut hasher = Hasher::new();
            let mut buf = [0u8; 4096];
            let mut fp = tmpfile.open(OpenOptions::new()
                .write(true).truncate(true).create(true))
                .map_err(|e| ("Can't open temporary file", e))?;
            let mut size = blob.read(&mut buf)
                .map_err(|e| ("Error reading input", e))?;
            while size > 0 {
                hasher.update(&buf[..size]);
                fp.write_all(&buf[..size])
                    .map_err(|e| ("Error writing to temporary file", e))?;
                size = blob.read(&mut buf)
                    .map_err(|e| ("Error reading input", e))?;
            }
            hasher.result()
        };

        let path = self.filename(&id);
        if !path.exists() {
            {
                let parent = path.parent().unwrap();
                if !parent.exists() {
                    fs::create_dir(parent)
                        .map_err(|e| ("Couldn't create blob directory", e))?;
                }
            }
            tmpfile.rename(path)
                .map_err(|e| ("Couldn't move temporary file to storage", e))?;
        }

        Ok(id)
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
            let mut id = [0u8; 32];
            id[..2].clone_from_slice(&self.first_val);
            let name = entry.file_name()
                .into_string()
                .expect("Second-level entry in blobs is invalid unicode");
            let slice = name.as_bytes();
            if slice.len() != 30 {
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
