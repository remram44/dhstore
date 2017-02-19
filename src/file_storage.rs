use std::fs::{File, OpenOptions};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use sha1::Sha1;

use common::{ID, EnumerableBlobStorage, BlobStorage};

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
    fn get_blob(&self, id: &ID) -> Option<Box<[u8]>> {
        let path = self.filename(id);
        if path.exists() {
            let mut fp = File::open(path).expect("Can't open blob file");
            let mut vec = Vec::new();
            fp.read_to_end(&mut vec).expect("Error reading blob file");
            Some(vec.into_boxed_slice())
        } else {
            None
        }
    }

    fn add_blob(&mut self, blob: &[u8]) -> ID {
        let mut sha1 = Sha1::new();
        sha1.update(blob);
        let id = ID { bytes: sha1.digest().bytes() };
        self.add_known_blob(&id, blob);
        id
    }

    fn add_known_blob(&mut self, id: &ID, blob: &[u8]) {
        let path = self.filename(id);
        if !path.exists() {
            let mut fp = OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(path)
                .expect("Can't open new blob file");
            fp.write_all(blob).expect("Error writing blob file");
        }
    }

    fn delete_blob(&mut self, id: &ID) {
        let path = self.filename(id);
        if path.exists() {
            ::std::fs::remove_file(path);
        }
    }
}

impl EnumerableBlobStorage for FileBlobStorage {
    type Iter = FileBlobIterator;

    fn list_blobs(&self) -> FileBlobIterator {
        let mut first = self.path
            .read_dir()
            .expect("Blobs directory doesn't exist");
        FileBlobIterator {
            first: first,
            first_val: [0u8; 2],
            second: None,
        }
    }
}

pub struct FileBlobIterator {
    first: ::std::fs::ReadDir,
    first_val: [u8; 2],
    second: Option<::std::fs::ReadDir>,
}

impl Iterator for FileBlobIterator {
    type Item = ID;

    fn next(&mut self) -> Option<ID> {
        if self.second.is_none() {
            if let Some(entry) = self.first.next() {
                let entry =
                    entry.expect("Error reading first-level entry in blobs");
                let name = entry.file_name()
                    .into_string()
                    .expect("First-level entry in blobs is invalid unicode");
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
            Some(ID { bytes: id })
        } else {
            self.second = None;
            self.next()
        }
    }
}
