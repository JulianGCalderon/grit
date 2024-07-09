use std::io::{self, BufRead, BufReader, Read, Write};

use flate2::{
    read::ZlibDecoder as ZlibReadDecoder, write::ZlibEncoder as ZlibWriteEncoder, Compression,
};
use sha1::{Digest, Sha1};

use crate::{repository::GitResult, utils::extract_bits};

use super::{Oid, RawOid};

#[derive(Default, PartialEq, Eq, Debug)]
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

    pub fn deserialize<R: Read>(reader: R) -> GitResult<Self> {
        let mut decoder = BufReader::new(ZlibReadDecoder::new(reader));

        let mut header_bytes = Vec::new();
        decoder.read_until(b'\0', &mut header_bytes)?;

        let mut entries = Vec::new();

        while let Ok(entry) = TreeEntry::deserialize(&mut decoder) {
            entries.push(entry);
        }

        Ok(Self::new(entries))
    }

    fn header(&self) -> Vec<u8> {
        let tree_size: usize = self.entries.iter().map(|entry| entry.size()).sum();
        format!("tree {}\0", tree_size).bytes().collect()
    }
}

#[derive(PartialEq, Eq, Debug)]
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

        writer.write_all(&file_type_1.to_be_bytes())?;
        writer.write_all(&file_type_2.to_be_bytes())?;
        writer.write_all(&special.to_be_bytes())?;
        writer.write_all(&owner.to_be_bytes())?;
        writer.write_all(&group.to_be_bytes())?;
        writer.write_all(&others.to_be_bytes())?;
        writer.write_all(&[b' '])?;
        writer.write_all(self.name.as_bytes())?;
        writer.write_all(&[b'\0'])?;
        writer.write_all(&self.oid.to_raw_bytes())?;

        Ok(())
    }

    pub fn deserialize<R: BufRead>(mut reader: R) -> GitResult<Self> {
        let mut file_type_1_byte = [0; 1];
        reader.read_exact(&mut file_type_1_byte)?;
        let file_type_1 = u8::from_be_bytes(file_type_1_byte) - b'0';

        let mut file_type_2_byte = [0; 1];
        reader.read_exact(&mut file_type_2_byte)?;
        let file_type_2 = u8::from_be_bytes(file_type_2_byte) - b'0';

        let mut special_byte = [0; 1];
        reader.read_exact(&mut special_byte)?;
        let special = u8::from_be_bytes(special_byte) - b'0';

        let mut owner_byte = [0; 1];
        reader.read_exact(&mut owner_byte)?;
        let owner = u8::from_be_bytes(owner_byte) - b'0';

        let mut group_byte = [0; 1];
        reader.read_exact(&mut group_byte)?;
        let group = u8::from_be_bytes(group_byte) - b'0';

        let mut others_byte = [0; 1];
        reader.read_exact(&mut others_byte)?;
        let others = u8::from_be_bytes(others_byte) - b'0';

        let mode = ((file_type_1 as u32) << 15)
            | ((file_type_2 as u32) << 12)
            | ((special as u32) << 9)
            | ((owner as u32) << 6)
            | ((group as u32) << 3)
            | (others as u32);

        let mut garbage = [0; 1];
        reader.read_exact(&mut garbage)?;

        let mut name = Vec::new();
        reader.read_until(b'\0', &mut name)?;
        name.pop();
        let name =
            String::from_utf8(name).map_err(|_| io::Error::from(io::ErrorKind::InvalidInput))?;

        let mut raw_oid = RawOid::default();
        reader.read_exact(&mut raw_oid)?;
        let oid = Oid::from_raw_bytes(raw_oid);

        Ok(Self { mode, name, oid })
    }

    pub fn size(&self) -> usize {
        // 28 bytes are fixed, entry name is variable
        28 + self.name.len()
    }
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use pretty_assertions_sorted::assert_eq;

    use super::*;

    #[test]
    pub fn can_serialize_and_deserialized() {
        let entries = vec![
            TreeEntry {
                mode: 1234,
                name: "name1".to_string(),
                oid: Oid::new("f0133c7517d34d37f8dca8c8444c6a9cdd7e4cdc").unwrap(),
            },
            TreeEntry {
                mode: 1234,
                name: "name2".to_string(),
                oid: Oid::new("f0133c7517d34d37f8dca8c8444c6a9cdd7e4cdc").unwrap(),
            },
            TreeEntry {
                mode: 1234,
                name: "name3".to_string(),
                oid: Oid::new("f0133c7517d34d37f8dca8c8444c6a9cdd7e4cdc").unwrap(),
            },
        ];
        let tree = Tree::new(entries);

        let mut serialized = Vec::new();
        tree.serialize(&mut serialized).unwrap();

        let serialized_cursor = Cursor::new(serialized);
        let deserialized = Tree::deserialize(serialized_cursor).unwrap();

        assert_eq!(tree, deserialized);
    }

    #[test]
    pub fn size_calculation_is_correct() {
        let entry = TreeEntry {
            mode: 1234,
            name: "name".to_string(),
            oid: Oid::new("f0133c7517d34d37f8dca8c8444c6a9cdd7e4cdc").unwrap(),
        };

        let mut serialized = Vec::new();
        entry.serialize(&mut serialized).unwrap();

        assert_eq!(serialized.len(), entry.size())
    }
}
