use std::io::{Read, Write};

use flate2::{write::ZlibEncoder as ZlibWriteEncoder, Compression};
use sha1::{Digest, Sha1};

use crate::{repository::GitResult, utils::extract_bits};

use super::Oid;

pub struct Tree {
    entries: Vec<TreeEntry>,
}

impl Tree {
    pub fn new(entries: Vec<TreeEntry>) -> Self {
        Self { entries }
    }

    pub fn hash(&self) -> Oid {
        let mut hasher = Sha1::new();
        let header = self.header();

        hasher.update(&header);

        for entry in &self.entries {
            entry
                .serialize(&mut hasher)
                .expect("writing to hasher cannot fail");
        }

        let raw_id = hasher.finalize().into();

        Oid::from_raw_bytes(raw_id)
    }

    pub fn serialize<W: Write>(&self, writer: W) -> GitResult<()> {
        let mut encoder = ZlibWriteEncoder::new(writer, Compression::default());

        encoder.write_all(&self.header())?;

        for entry in &self.entries {
            entry.serialize(&mut encoder)?;
        }

        Ok(())
    }

    pub fn deserialize<R: Read>(&self, _reader: R) -> GitResult<()> {
        todo!()
    }

    fn header(&self) -> Vec<u8> {
        let tree_size: usize = self.entries.iter().map(|entry| entry.size()).sum();
        format!("tree {}\0", tree_size).bytes().collect()
    }
}

pub struct TreeEntry {
    mode: u32,
    name: String,
    oid: Oid,
}

impl TreeEntry {
    pub fn new(mode: u32, name: String, oid: Oid) -> GitResult<Self> {
        Ok(Self { mode, name, oid })
    }

    pub fn serialize<W: Write>(&self, mut writer: W) -> GitResult<()> {
        let file_type_1 = extract_bits(self.mode, 0o100000, 15) as u8 + b'0';
        let file_type_2 = extract_bits(self.mode, 0o70000, 12) as u8 + b'0';
        let special = extract_bits(self.mode, 0o7000, 9) as u8 + b'0';
        let owner = extract_bits(self.mode, 0o700, 6) as u8 + b'0';
        let group = extract_bits(self.mode, 0o70, 3) as u8 + b'0';
        let others = extract_bits(self.mode, 0o7, 0) as u8 + b'0';

        writer.write(&file_type_1.to_be_bytes())?;
        writer.write(&file_type_2.to_be_bytes())?;
        writer.write(&special.to_be_bytes())?;
        writer.write(&owner.to_be_bytes())?;
        writer.write(&group.to_be_bytes())?;
        writer.write(&others.to_be_bytes())?;
        writer.write(&[b' '])?;
        writer.write(self.name.as_bytes())?;
        writer.write(&[b'\0'])?;
        writer.write(&self.oid.to_raw_bytes())?;

        Ok(())
    }

    pub fn deserialize<R: Read>(&self, _reader: R) -> GitResult<()> {
        todo!()
    }

    pub fn size(&self) -> usize {
        // 28 bytes are fixed, the only variable element is the entry name
        28 + self.name.len()
    }
}

#[cfg(test)]
mod tests {

    #[test]
    pub fn can_serialize_and_deserialized() {}

    #[test]
    pub fn size_is_correct() {}
}
