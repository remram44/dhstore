extern crate dhstore;

fn main() {
    // Create a file blob storage, storing blobs as single files
    let storage = dhstore::FileBlobStorage::open("store/blobs");

    // Create a memory index, that stores all the objects in memory, and
    // has to load all of them everytime from simple files
    let index = dhstore::MemoryIndex::open("store/objects");

    // Create the Store object
    let store = dhstore::Store::open(storage, index);
}
