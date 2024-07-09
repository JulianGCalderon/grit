use std::{
    fs::{File, Metadata},
    io::{BufRead, BufReader, Write},
    os::unix::fs::MetadataExt,
    path::Path,
};

use sha1::{Digest, Sha1};

use crate::{
    object::{Oid, RawOid},
    repository::GitResult,
    utils::extract_bits,
};

const INDEX_SIGNATURE: &str = "DIRC";
const INDEX_VERSION: u32 = 2;

#[derive(Default, PartialEq, Eq, Debug)]
pub struct Index {
    entries: Vec<IndexEntry>,
}

impl Index {
    pub fn new(mut entries: Vec<IndexEntry>) -> Self {
        entries.sort();

        Self { entries }
    }

    pub fn entries(&self) -> &[IndexEntry] {
        &self.entries
    }

    pub fn push(&mut self, entry: IndexEntry) {
        match self
            .entries
            .binary_search_by_key(&entry.name.as_str(), |entry| entry.name.as_str())
        {
            Ok(pos) => self.entries[pos] = entry,
            Err(pos) => self.entries.insert(pos, entry),
        }
    }

    pub fn serialize_to_path<P: AsRef<Path>>(&self, path: P) -> GitResult<()> {
        let file = File::create(path)?;
        self.serialize(file)
    }

    pub fn serialize<W: Write>(&self, mut writer: W) -> GitResult<()> {
        let mut hasher = Sha1::new();

        writer.write_all(INDEX_SIGNATURE.as_bytes())?;
        hasher.write_all(INDEX_SIGNATURE.as_bytes())?;

        writer.write_all(&INDEX_VERSION.to_be_bytes())?;
        hasher.write_all(&INDEX_VERSION.to_be_bytes())?;

        writer.write_all(&(self.entries.len() as u32).to_be_bytes())?;
        hasher.write_all(&(self.entries.len() as u32).to_be_bytes())?;

        for entry in &self.entries {
            entry.serialize(&mut writer)?;
            entry.serialize(&mut hasher)?;
        }

        let hash = hasher.finalize();
        writer.write_all(&hash)?;

        Ok(())
    }

    pub fn deserialize_from_path<P: AsRef<Path>>(path: P) -> GitResult<Self> {
        let file = BufReader::new(File::open(path)?);
        Self::deserialize(file)
    }

    pub fn deserialize<R: BufRead>(mut reader: R) -> GitResult<Self> {
        let mut signature_bytes = [0; 4];
        reader.read_exact(&mut signature_bytes)?;
        let mut version_bytes = [0; 4];
        reader.read_exact(&mut version_bytes)?;
        let mut length_bytes = [0; 4];
        reader.read_exact(&mut length_bytes)?;

        let length = u32::from_be_bytes(length_bytes);

        let mut entries = Vec::with_capacity(length as usize);

        for _ in 0..length {
            let entry = IndexEntry::deserialize(&mut reader)?;
            entries.push(entry);
        }

        Ok(Index::new(entries))
    }
}

#[derive(PartialEq, Eq, Clone, Debug)]
pub struct IndexEntry {
    ctime: i32,
    ctime_nsec: i32,
    mtime: i32,
    mtime_nsec: i32,
    dev: u32,
    ino: u32,
    mode: u32,
    uid: u32,
    gid: u32,
    size: u32,
    oid: Oid,
    assume_valid: bool,
    stage: u8,
    name: String,
}

impl IndexEntry {
    pub fn new(
        metadata: Metadata,
        oid: Oid,
        assume_valid: bool,
        stage: u8,
        name: String,
    ) -> GitResult<Self> {
        Ok(Self {
            ctime: metadata.ctime() as i32,
            ctime_nsec: metadata.ctime_nsec() as i32,
            mtime: metadata.mtime() as i32,
            mtime_nsec: metadata.mtime_nsec() as i32,
            dev: metadata.dev() as u32,
            ino: metadata.ino() as u32,
            mode: metadata.mode() as u32,
            uid: metadata.uid() as u32,
            gid: metadata.gid() as u32,
            size: metadata.size() as u32,
            oid,
            assume_valid,
            stage,
            name,
        })
    }

    pub fn serialize<W: Write>(&self, mut writer: W) -> GitResult<()> {
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

        writer.write_all(&self.oid.to_raw_bytes())?;

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

    pub fn deserialize<R: BufRead>(mut reader: R) -> GitResult<Self> {
        let mut ctime_bytes = [0; 4];
        reader.read_exact(&mut ctime_bytes)?;
        let ctime = i32::from_be_bytes(ctime_bytes);

        let mut ctime_nsec_bytes = [0; 4];
        reader.read_exact(&mut ctime_nsec_bytes)?;
        let ctime_nsec = i32::from_be_bytes(ctime_nsec_bytes);

        let mut mtime_bytes = [0; 4];
        reader.read_exact(&mut mtime_bytes)?;
        let mtime = i32::from_be_bytes(mtime_bytes);

        let mut mtime_nsec_bytes = [0; 4];
        reader.read_exact(&mut mtime_nsec_bytes)?;
        let mtime_nsec = i32::from_be_bytes(mtime_nsec_bytes);

        let mut dev_bytes = [0; 4];
        reader.read_exact(&mut dev_bytes)?;
        let dev = u32::from_be_bytes(dev_bytes);

        let mut ino_bytes = [0; 4];
        reader.read_exact(&mut ino_bytes)?;
        let ino = u32::from_be_bytes(ino_bytes);

        let mut mode_bytes = [0; 4];
        reader.read_exact(&mut mode_bytes)?;
        let mode = u32::from_be_bytes(mode_bytes);

        let mut uid_bytes = [0; 4];
        reader.read_exact(&mut uid_bytes)?;
        let uid = u32::from_be_bytes(uid_bytes);

        let mut gid_bytes = [0; 4];
        reader.read_exact(&mut gid_bytes)?;
        let gid = u32::from_be_bytes(gid_bytes);

        let mut size_bytes = [0; 4];
        reader.read_exact(&mut size_bytes)?;
        let size = u32::from_be_bytes(size_bytes);

        let mut oid_bytes = RawOid::default();
        reader.read_exact(&mut oid_bytes)?;
        let oid = Oid::from_raw_bytes(oid_bytes);

        let mut flags_bytes = [0; 2];
        reader.read_exact(&mut flags_bytes)?;
        let flags = u16::from_be_bytes(flags_bytes);
        let assume_valid = extract_bits(flags, 0b1, 15) != 0;
        let stage = extract_bits(flags, 0b11, 12) as u8;
        let _name_length = extract_bits(flags, 0xFFF, 0);

        let mut name_bytes = Vec::new();
        reader.read_until(b'\0', &mut name_bytes)?;
        // the null terminator is read, so we remove it and take it into account when reading padding
        name_bytes.pop();
        let name = String::from_utf8(name_bytes).unwrap_or_default();

        // entry size must be multiple of 8
        // - first 10 fields occupy 4 bytes each: offset = 0
        // - hash always occupies 20 bytes: offset = 4
        // - flags occupy 2 bytes: offset = 2
        // - name is variable length: offset = ?
        let offset = (4 + 2 + name.len()) % 8;
        // we use 7 instead of 8 as we already read the string null terminator
        let mut padding_bytes = vec![0; (7 - offset) as usize];
        reader.read_exact(&mut padding_bytes)?;

        Ok(IndexEntry {
            ctime,
            ctime_nsec,
            mtime,
            mtime_nsec,
            dev,
            ino,
            mode,
            uid,
            gid,
            size,
            oid,
            assume_valid,
            stage,
            name,
        })
    }

    pub fn ctime(&self) -> i32 {
        self.ctime
    }
    pub fn ctime_nsec(&self) -> i32 {
        self.ctime_nsec
    }
    pub fn mtime(&self) -> i32 {
        self.mtime
    }
    pub fn mtime_nsec(&self) -> i32 {
        self.mtime_nsec
    }
    pub fn dev(&self) -> u32 {
        self.dev
    }
    pub fn ino(&self) -> u32 {
        self.ino
    }
    pub fn mode(&self) -> u32 {
        self.mode
    }
    pub fn uid(&self) -> u32 {
        self.uid
    }
    pub fn gid(&self) -> u32 {
        self.gid
    }
    pub fn size(&self) -> u32 {
        self.size
    }
    pub fn oid(&self) -> &Oid {
        &self.oid
    }
    pub fn assume_valid(&self) -> bool {
        self.assume_valid
    }
    pub fn stage(&self) -> u8 {
        self.stage
    }
    pub fn name(&self) -> &str {
        &self.name
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

    use pretty_assertions_sorted::assert_eq;

    #[test]
    pub fn can_serialize_and_deserialize() {
        let entries = vec![
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
                oid: Oid::new("f0133c7517d34d37f8dca8c8444c6a9cdd7e4cdc").unwrap(),
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
                oid: Oid::new("554b0c91f951764bb11f1db849685d95b2c7a48f").unwrap(),
                assume_valid: true,
                stage: 1,
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
                oid: Oid::new("bedc28ca5099946b354104a3c6cc90ec20dbcaec").unwrap(),
                assume_valid: false,
                stage: 2,
                name: "name3".to_string(),
            },
        ];
        let index = Index::new(entries);

        let mut serialized = Vec::new();
        index.serialize(&mut serialized).unwrap();

        let serialized_cursor = Cursor::new(serialized);
        let deserialized = Index::deserialize(serialized_cursor).unwrap();

        assert_eq!(index, deserialized);
    }
}
