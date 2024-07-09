use std::io::{Read, Write};

use flate2::{write::ZlibEncoder as ZlibWriteEncoder, Compression};
use sha1::{Digest, Sha1};

use crate::{repository::GitResult, utils::extract_bits};

use super::{Oid, OID_HEX_LEN};

pub struct Tree {
    pub entries: Vec<TreeEntry>,
}

pub struct TreeEntry {
    pub mode: u32,
    pub name: String,
    pub oid: Oid,
}

impl Tree {
    pub fn hash(&self) -> Oid {
        let mut hasher = Sha1::new();
        let header = self.header();

        hasher.update(&header);

        for entry in &self.entries {
            entry
                .serialize(&mut hasher)
                .expect("writing to hasher cannot fail");
        }

        let raw_id: [u8; OID_HEX_LEN] = hasher.finalize().into();

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

impl TreeEntry {
    pub fn serialize<W: Write>(&self, mut writer: W) -> GitResult<()> {
        let file_type_1 = extract_bits(self.mode, 0b1, 15) as u8 + b'0';
        let file_type_2 = extract_bits(self.mode, 0o7, 12) as u8 + b'0';
        let special = extract_bits(self.mode, 0o7, 9) as u8 + b'0';
        let owner = extract_bits(self.mode, 0o7, 6) as u8 + b'0';
        let group = extract_bits(self.mode, 0o7, 3) as u8 + b'0';
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
