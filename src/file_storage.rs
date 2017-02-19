use std::path::{Path, PathBuf};

use common::{ID, EnumerableBlobStorage, BlobStorage};

pub struct FileBlobStorage {
    path: PathBuf,
}

impl FileBlobStorage {
    pub fn open<P: AsRef<Path>>(path: P) -> FileBlobStorage {
        FileBlobStorage {
            path: path.as_ref().to_path_buf(),
        }
    }
}

impl BlobStorage for FileBlobStorage {
    fn get_blob(&self, id: &ID) -> Option<Box<[u8]>> {
        unimplemented!()
    }

    fn add_blob(&mut self, blob: &[u8]) -> ID {
        unimplemented!()
    }

    fn delete_blob(&mut self, id: &ID) {
        unimplemented!()
    }
}

impl EnumerableBlobStorage for FileBlobStorage {
}
