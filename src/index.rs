use std::{
    fs::File,
    io::{self, Read, Write},
    path::Path,
};

use sha1::{Digest, Sha1};

use crate::{command::GitResult, object::OId};

const INDEX_SIGNATURE: &str = "DIRC";
const INDEX_VERSION: u32 = 2;

#[derive(Default, PartialEq, Eq, Debug)]
pub struct Index {
    pub entries: Vec<IndexEntry>,
}

#[derive(PartialEq, Eq, Clone, Debug)]
pub struct IndexEntry {
    pub ctime: i32,
    pub ctime_nsec: i32,
    pub mtime: i32,
    pub mtime_nsec: i32,
    pub dev: u32,
    pub ino: u32,
    pub mode: u32,
    pub uid: u32,
    pub gid: u32,
    pub size: u32,
    pub oid: OId,
    // flags
    pub assume_valid: bool,
    pub stage: u8,
    //
    pub name: String,
}

impl Index {
    pub fn serialize<P: AsRef<Path>>(&self, path: P) -> GitResult<()> {
        let file = File::create(path)?;
        self.serialize_to_writer(file)
    }

    pub fn serialize_to_writer<W: Write>(&self, mut writer: W) -> GitResult<()> {
        let mut hasher = Sha1::new();

        writer.write_all(INDEX_SIGNATURE.as_bytes())?;
        hasher.write_all(INDEX_SIGNATURE.as_bytes())?;

        writer.write_all(&INDEX_VERSION.to_be_bytes())?;
        hasher.write_all(&INDEX_VERSION.to_be_bytes())?;

        writer.write_all(&(self.entries.len() as u32).to_be_bytes())?;
        hasher.write_all(&(self.entries.len() as u32).to_be_bytes())?;

        let mut entries = self.entries.clone();
        entries.sort();

        for entry in &self.entries {
            entry.serialize_to_writer(&mut writer)?;
            entry.serialize_to_writer(&mut hasher)?;
        }

        let hash = hasher.finalize();
        writer.write_all(&hash)?;

        Ok(())
    }

    pub fn deserialize<P: AsRef<Path>>(path: P) -> GitResult<Self> {
        let file = File::open(path)?;
        Self::deserialize_from_reader(file)
    }

    pub fn deserialize_from_reader<R: Read>(_reader: R) -> GitResult<Self> {
        Err(io::Error::new(io::ErrorKind::Other, "not implemented"))?
    }
}

impl IndexEntry {
    pub fn serialize_to_writer<W: Write>(&self, mut writer: W) -> GitResult<()> {
        writer.write_all(&self.ctime.to_be_bytes())?;
        writer.write_all(&self.ctime_nsec.to_be_bytes())?;
        writer.write_all(&self.mtime.to_be_bytes())?;
        writer.write_all(&self.mtime_nsec.to_be_bytes())?;
        writer.write_all(&self.dev.to_be_bytes())?;
        writer.write_all(&self.ino.to_be_bytes())?;
        writer.write_all(&self.mode.to_be_bytes())?;
        writer.write_all(&self.uid.to_be_bytes())?;
        writer.write_all(&self.gid.to_be_bytes())?;
        writer.write_all(&self.size.to_be_bytes())?;

        let hash = base16ct::lower::decode_vec(&self.oid).expect("oid should be always lower hexa");
        writer.write_all(&hash)?;

        let assume_valid_bit = (self.assume_valid as u16) << 15;
        let extended_flag_bit = 0 << 14;
        let stage_bits = (self.stage.min(0b11) as u16) << 12;
        let name_length_as_u12 = self.name.len().min(0xFFF) as u16;
        let flags = assume_valid_bit | extended_flag_bit | stage_bits | name_length_as_u12;

        writer.write_all(&flags.to_be_bytes())?;
        writer.write_all(&self.name.as_bytes())?;

        // entry size must be multiple of 8
        // - first 10 fields occupy 4 bytes each: offset = 0
        // - hash always occupies 20 bytes: offset = 4
        // - flags occupy 2 bytes: offset = 2
        // - name is variable length: offset = ?
        let offset = (4 + 2 + self.name.len()) % 8;
        let padding = vec![0; (8 - offset) as usize];
        writer.write_all(&padding)?;

        Ok(())
    }

    pub fn deserialize_from_reader<R: Read>(_reader: R) -> GitResult<()> {
        Ok(())
    }
}

impl PartialOrd for IndexEntry {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for IndexEntry {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        let stage_ordering = self.stage.cmp(&other.stage);
        let name_ordering = self.name.cmp(&other.name);

        name_ordering.then(stage_ordering)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    pub fn index_integration() {
        let index = Index {
            entries: vec![
                IndexEntry {
                    ctime: 1234,
                    ctime_nsec: 1234,
                    mtime: 1234,
                    mtime_nsec: 1234,
                    dev: 1234,
                    ino: 1234,
                    mode: 1234,
                    uid: 1234,
                    gid: 1234,
                    size: 1234,
                    oid: "f0133c7517d34d37f8dca8c8444c6a9cdd7e4cdc".to_string(),
                    assume_valid: false,
                    stage: 0,
                    name: "name1".to_string(),
                },
                IndexEntry {
                    ctime: 4321,
                    ctime_nsec: 4321,
                    mtime: 4321,
                    mtime_nsec: 4321,
                    dev: 4321,
                    ino: 4321,
                    mode: 4321,
                    uid: 4321,
                    gid: 4321,
                    size: 4321,
                    oid: "554b0c91f951764bb11f1db849685d95b2c7a48f".to_string(),
                    assume_valid: false,
                    stage: 0,
                    name: "name2".to_string(),
                },
                IndexEntry {
                    ctime: 5678,
                    ctime_nsec: 5678,
                    mtime: 5678,
                    mtime_nsec: 5678,
                    dev: 5678,
                    ino: 5678,
                    mode: 5678,
                    uid: 5678,
                    gid: 5678,
                    size: 5678,
                    oid: "bedc28ca5099946b354104a3c6cc90ec20dbcaec".to_string(),
                    assume_valid: false,
                    stage: 0,
                    name: "name3".to_string(),
                },
            ],
        };

        let mut serialized = Vec::new();
        index.serialize_to_writer(&mut serialized).unwrap();

        let serialized_cursor = Cursor::new(serialized);
        let deserialized = Index::deserialize_from_reader(serialized_cursor).unwrap();

        assert_eq!(index, deserialized);
    }
}
