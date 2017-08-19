-- All objects in the system
--
-- 'type' is either 0 for a dict or 1 for a list.
--
-- 'gc' is used for concurrent garbage collection:
--   0 is the 'white' set, objects that are condemned to be removed.
--   1 is the 'gray' set, objects that are reachable from the root but that
--     we need to check.
--   2 is the 'black' set, objects that have no references to 'white' objects
--     and they we are keeping.
--   There is no reference to white objects from black objects.
--   New objects are created gray.
--   When the set of gray objects become empty, the white objects are dropped.
CREATE TABLE objects(
    id VARCHAR(40) NOT NULL PRIMARY KEY,
    type INTEGER NOT NULL,
    gc INTEGER NULL,
    valid_permanode BOOLEAN NOT NULL DEFAULT FALSE);
CREATE INDEX idx_objects_gc ON objects(gc);

-- The properties associated to objects
CREATE TABLE properties(
    object VARCHAR(40) NOT NULL,
    key VARCHAR(256) NOT NULL,
    value_int INTEGER NULL,
    value_str TEXT NULL,
    value_object VARCHAR(40) NULL,
    value_blob VARCHAR(40) NULL);
CREATE INDEX idx_properties_object ON properties(object);
CREATE INDEX idx_properties_key ON properties(key);
CREATE INDEX idx_properties_value_int ON properties(value_int);
CREATE INDEX idx_properties_value_str ON properties(value_str);
CREATE INDEX idx_properties_value_object ON properties(value_object);
CREATE INDEX idx_properties_value_blob ON properties(value_blob);

-- The valid claims for permanodes
CREATE TABLE claims(
    claim VARCHAR(40) NOT NULL PRIMARY KEY,
    permanode VARCHAR(40) NOT NULL);

-- The valid policies
CREATE TABLE policies(
    object VARCHAR(40) NOT NULL,
    policy VARCHAR(40) NOT NULL);
