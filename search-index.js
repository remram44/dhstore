var searchIndex = {};
searchIndex["dhstore"] = {"doc":"DHStore: A personal content management system.","items":[[3,"Object","dhstore","A schema object, i.e. either a dictionary or a list of properties.",null,null],[12,"id","","",0,null],[12,"data","","",0,null],[3,"MemoryIndex","","The in-memory index, that loads all objects from the disk on startup.",null,null],[3,"FileBlobStorage","","Filesystem-based blob storage implementation.",null,null],[3,"Store","","Main structure, representing the whole system.",null,null],[4,"Property","","Values that appear in an object's metadata.",null,null],[13,"String","","",1,null],[13,"Integer","","",1,null],[13,"Reference","","",1,null],[13,"Blob","","",1,null],[4,"ObjectData","","The types of object known to the index.",null,null],[13,"Dict","","",2,null],[13,"List","","",2,null],[5,"permanode","","",null,{"inputs":[{"name":"dict"},{"name":"sort"}],"output":{"name":"object"}}],[5,"open","","Opens a directory.",null,{"inputs":[{"name":"p"}],"output":{"generics":["store"],"name":"result"}}],[5,"create","","Creates a new store on disk.",null,{"inputs":[{"name":"p"}],"output":{"name":"result"}}],[11,"clone","","",1,{"inputs":[{"name":"self"}],"output":{"name":"property"}}],[11,"fmt","","",1,{"inputs":[{"name":"self"},{"name":"formatter"}],"output":{"name":"result"}}],[11,"eq","","",1,{"inputs":[{"name":"self"},{"name":"property"}],"output":{"name":"bool"}}],[11,"ne","","",1,{"inputs":[{"name":"self"},{"name":"property"}],"output":{"name":"bool"}}],[11,"partial_cmp","","",1,{"inputs":[{"name":"self"},{"name":"property"}],"output":{"generics":["ordering"],"name":"option"}}],[11,"cmp","","",1,{"inputs":[{"name":"self"},{"name":"property"}],"output":{"name":"ordering"}}],[0,"errors","","Error definitions",null,null],[4,"Error","dhstore::errors","An error from dhstore.",null,null],[13,"IoError","","",3,null],[13,"CorruptedStore","","",3,null],[13,"InvalidInput","","",3,null],[6,"Result","","Alias for the `Result` type with an error of our `Error` type.",null,null],[11,"fmt","","",3,{"inputs":[{"name":"self"},{"name":"formatter"}],"output":{"name":"result"}}],[11,"fmt","","",3,{"inputs":[{"name":"self"},{"name":"formatter"}],"output":{"generics":["error"],"name":"result"}}],[11,"description","","",3,{"inputs":[{"name":"self"}],"output":{"name":"str"}}],[11,"cause","","",3,{"inputs":[{"name":"self"}],"output":{"generics":["error"],"name":"option"}}],[11,"from","","",3,null],[11,"open","dhstore","Opens the blob storage from a path.",4,{"inputs":[{"name":"p"}],"output":{"name":"fileblobstorage"}}],[11,"get_blob","","",4,{"inputs":[{"name":"self"},{"name":"id"}],"output":{"generics":["option"],"name":"result"}}],[11,"add_blob","","",4,null],[11,"add_known_blob","","",4,null],[11,"delete_blob","","",4,{"inputs":[{"name":"self"},{"name":"id"}],"output":{"name":"result"}}],[11,"verify","","",4,{"inputs":[{"name":"self"}],"output":{"name":"result"}}],[11,"list_blobs","","",4,{"inputs":[{"name":"self"}],"output":{"generics":["fileblobiterator"],"name":"result"}}],[0,"hash","","Structures and functions related to hashing.",null,null],[3,"ID","dhstore::hash","Identifier for an object.",null,null],[12,"bytes","","",5,null],[3,"Hasher","","Content to ID code.",null,null],[3,"HasherWriter","","A convenient adapter to hash while writing.",null,null],[3,"HasherReader","","A convenient adapter to hash while reading.",null,null],[17,"HASH_SIZE","","Size of the hash in bytes.",null,null],[11,"clone","","",5,{"inputs":[{"name":"self"}],"output":{"name":"id"}}],[11,"eq","","",5,{"inputs":[{"name":"self"},{"name":"id"}],"output":{"name":"bool"}}],[11,"ne","","",5,{"inputs":[{"name":"self"},{"name":"id"}],"output":{"name":"bool"}}],[11,"partial_cmp","","",5,{"inputs":[{"name":"self"},{"name":"id"}],"output":{"generics":["ordering"],"name":"option"}}],[11,"lt","","",5,{"inputs":[{"name":"self"},{"name":"id"}],"output":{"name":"bool"}}],[11,"le","","",5,{"inputs":[{"name":"self"},{"name":"id"}],"output":{"name":"bool"}}],[11,"gt","","",5,{"inputs":[{"name":"self"},{"name":"id"}],"output":{"name":"bool"}}],[11,"ge","","",5,{"inputs":[{"name":"self"},{"name":"id"}],"output":{"name":"bool"}}],[11,"cmp","","",5,{"inputs":[{"name":"self"},{"name":"id"}],"output":{"name":"ordering"}}],[11,"from_slice","","Make an ID from raw bytes.",5,null],[11,"str","","Returns a string representation of the ID.",5,{"inputs":[{"name":"self"}],"output":{"name":"string"}}],[11,"from_str","","Parses the string representation into a ID.",5,null],[11,"hash","","",5,{"inputs":[{"name":"self"},{"name":"h"}],"output":null}],[11,"fmt","","",5,{"inputs":[{"name":"self"},{"name":"formatter"}],"output":{"generics":["error"],"name":"result"}}],[11,"fmt","","",5,{"inputs":[{"name":"self"},{"name":"formatter"}],"output":{"generics":["error"],"name":"result"}}],[11,"default","","",6,{"inputs":[],"output":{"name":"hasher"}}],[11,"new","","Build a new `Hasher`.",6,{"inputs":[],"output":{"name":"hasher"}}],[11,"result","","Consume this `Hasher` and return an `ID`.",6,{"inputs":[{"name":"self"}],"output":{"name":"id"}}],[11,"write","","",6,null],[11,"flush","","",6,{"inputs":[{"name":"self"}],"output":{"name":"result"}}],[11,"new","","Build a new `HasherWriter` that will write on the given object.",7,{"inputs":[{"name":"w"}],"output":{"name":"hasherwriter"}}],[11,"with_hasher","","",7,{"inputs":[{"name":"w"},{"name":"hasher"}],"output":{"name":"hasherwriter"}}],[11,"result","","Consume this object and returns the `ID` computed from hashing.",7,{"inputs":[{"name":"self"}],"output":{"name":"id"}}],[11,"write","","",7,null],[11,"flush","","",7,{"inputs":[{"name":"self"}],"output":{"name":"result"}}],[11,"write_all","","",7,null],[11,"new","","Build a new `HasherReader` that will read from the given object.",8,{"inputs":[{"name":"r"}],"output":{"name":"hasherreader"}}],[11,"with_hasher","","",8,{"inputs":[{"name":"r"},{"name":"hasher"}],"output":{"name":"hasherreader"}}],[11,"result","","Consume this object and returns the `ID` computed from hashing.",8,{"inputs":[{"name":"self"}],"output":{"name":"id"}}],[11,"read","","",8,null],[0,"log","dhstore","Log utilities.",null,null],[5,"init","dhstore::log","Sets up the logger object to log on stderr with the given log level.",null,{"inputs":[{"name":"loglevel"}],"output":{"generics":["setloggererror"],"name":"result"}}],[11,"open","dhstore","Reads all the objects from a directory into memory.",9,{"inputs":[{"name":"p"},{"name":"id"}],"output":{"generics":["memoryindex"],"name":"result"}}],[11,"create","","",9,{"inputs":[{"name":"p"},{"name":"i"}],"output":{"name":"result"}}],[11,"add","","",9,{"inputs":[{"name":"self"},{"name":"objectdata"}],"output":{"generics":["id"],"name":"result"}}],[11,"get_object","","",9,{"inputs":[{"name":"self"},{"name":"id"}],"output":{"generics":["option"],"name":"result"}}],[11,"verify","","",9,{"inputs":[{"name":"self"}],"output":{"name":"result"}}],[11,"collect_garbage","","",9,{"inputs":[{"name":"self"}],"output":{"generics":["hashset"],"name":"result"}}],[6,"Dict","","",null,null],[6,"List","","",null,null],[8,"BlobStorage","","Trait for the blob storage backends, that handle the specifics of storing blobs. A blob is an unnamed sequence of bytes, which constitute parts of some file's contents.",null,null],[10,"get_blob","","Gets a blob from its ID.",10,{"inputs":[{"name":"self"},{"name":"id"}],"output":{"generics":["option"],"name":"result"}}],[10,"add_blob","","Hashes a blob then adds it to the store.",10,null],[10,"add_known_blob","","Adds a blob whose hash is already known.",10,null],[10,"delete_blob","","Deletes a blob from its hash.",10,{"inputs":[{"name":"self"},{"name":"id"}],"output":{"name":"result"}}],[10,"verify","","Checks the blob storage for errors.",10,{"inputs":[{"name":"self"}],"output":{"name":"result"}}],[8,"EnumerableBlobStorage","","Additional trait for a `BlobStorage` that knows how to enumerate all the blobs it has.",null,null],[16,"Iter","","",11,null],[10,"list_blobs","","Returns an iterator over the blobs in this store.",11,{"inputs":[{"name":"self"}],"output":{"name":"result"}}],[11,"collect_garbage","","Removes the blobs whose hash are not in the given set.",11,{"inputs":[{"name":"self"},{"generics":["id"],"name":"hashset"}],"output":{"name":"result"}}],[8,"ObjectIndex","","Trait for the index of schema objects.",null,null],[10,"add","","Hashes an object and adds it to the index.",12,{"inputs":[{"name":"self"},{"name":"objectdata"}],"output":{"generics":["id"],"name":"result"}}],[10,"get_object","","Gets an object from its hash.",12,{"inputs":[{"name":"self"},{"name":"id"}],"output":{"generics":["option"],"name":"result"}}],[10,"verify","","Checks the index for errors.",12,{"inputs":[{"name":"self"}],"output":{"name":"result"}}],[10,"collect_garbage","","Deletes unreferenced objects and returns the set of blobs to keep.",12,{"inputs":[{"name":"self"}],"output":{"generics":["hashset"],"name":"result"}}],[11,"new","","Creates a store from a given blob storage and object index.",13,{"inputs":[{"name":"s"},{"name":"i"}],"output":{"name":"store"}}],[11,"add_blob","","Low-level; adds a blob to the blob storage.",13,{"inputs":[{"name":"self"},{"name":"r"}],"output":{"generics":["id"],"name":"result"}}],[11,"get_blob","","Low-level; gets a single blob from the blob storage.",13,{"inputs":[{"name":"self"},{"name":"id"}],"output":{"generics":["option"],"name":"result"}}],[11,"get_object","","Low-level; gets a single object from the index by its ID.",13,{"inputs":[{"name":"self"},{"name":"id"}],"output":{"generics":["option"],"name":"result"}}],[11,"add_file","","Cuts a file into chunks and add a list object of them to the index.",13,{"inputs":[{"name":"self"},{"name":"r"}],"output":{"name":"result"}}],[11,"add","","Adds a file or directory recursively, representing directories as dicts and files as lists of blobs.",13,{"inputs":[{"name":"self"},{"name":"p"}],"output":{"generics":["id"],"name":"result"}}],[11,"verify","","Checks the blobs and objects for errors.",13,{"inputs":[{"name":"self"}],"output":{"name":"result"}}],[11,"print_object","","Pretty-prints objects recursively.",13,{"inputs":[{"name":"self"},{"name":"id"},{"generics":["usize"],"name":"option"}],"output":{"name":"result"}}],[11,"collect_garbage","","",13,{"inputs":[{"name":"self"}],"output":{"name":"result"}}],[11,"collect_garbage","","Removes the blobs whose hash are not in the given set.",11,{"inputs":[{"name":"self"},{"generics":["id"],"name":"hashset"}],"output":{"name":"result"}}]],"paths":[[3,"Object"],[4,"Property"],[4,"ObjectData"],[4,"Error"],[3,"FileBlobStorage"],[3,"ID"],[3,"Hasher"],[3,"HasherWriter"],[3,"HasherReader"],[3,"MemoryIndex"],[8,"BlobStorage"],[8,"EnumerableBlobStorage"],[8,"ObjectIndex"],[3,"Store"]]};
initSearch(searchIndex);
