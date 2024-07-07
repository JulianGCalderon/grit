use std::{
    fs::File,
    io::{BufRead, BufReader, Read, Write},
    path::Path,
};

use flate2::{
    read::ZlibDecoder as ZlibReadDecoder, write::ZlibEncoder as ZlibWriteEncoder, Compression,
};
use sha1::{Digest, Sha1};

use crate::command::GitResult;

pub type OId = String;

pub struct Blob {
    content: Vec<u8>,
    id: OId,
}

impl Blob {
    pub fn create<P: AsRef<Path>>(path: P) -> GitResult<Self> {
        let file = File::open(path)?;
        let reader = BufReader::new(file);

        Self::create_from_reader(reader)
    }

    pub fn create_from_reader<R: Read>(mut reader: R) -> GitResult<Self> {
        let mut content = Vec::new();
        reader.read_to_end(&mut content)?;

        let id = Self::calculate_hash(&content);

        Ok(Self { content, id })
    }

    pub fn save<P: AsRef<Path>>(&self, path: P) -> GitResult<()> {
        let file = File::create(path)?;
        self.save_to_writer(file)
    }

    pub fn save_to_writer<W: Write>(&self, writer: W) -> GitResult<()> {
        let mut encoder = ZlibWriteEncoder::new(writer, Compression::default());
        encoder.write_all(&Self::create_header(self.size()))?;
        encoder.write_all(&self.content)?;

        Ok(())
    }

    pub fn read<P: AsRef<Path>>(path: P) -> GitResult<Self> {
        let file = File::open(path)?;
        Self::read_from_reader(file)
    }

    pub fn read_from_reader<R: Read>(reader: R) -> GitResult<Self> {
        let mut decoder = BufReader::new(ZlibReadDecoder::new(reader));

        let mut object_header = Vec::new();
        decoder.read_until(0, &mut object_header)?;

        let mut content = Vec::new();
        decoder.read_to_end(&mut content)?;

        let id = Self::calculate_hash(&content);

        Ok(Self { content, id })
    }

    pub fn id(&self) -> &OId {
        &self.id
    }

    pub fn size(&self) -> usize {
        self.content.len()
    }

    pub fn content(&self) -> &[u8] {
        &self.content
    }

    fn create_header(size: usize) -> Vec<u8> {
        format!("blob {size}\0").bytes().collect()
    }

    fn calculate_hash(content: &[u8]) -> String {
        let mut hasher = Sha1::new();
        let header = Self::create_header(content.len());
        hasher.update(&header);
        hasher.update(&content);
        base16ct::lower::encode_string(&hasher.finalize())
    }
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use rand::{RngCore, SeedableRng};

    use super::*;

    #[test]
    pub fn blob_fuzzy_integration() {
        let mut content = vec![0; 100];
        let mut rng_core = rand::rngs::StdRng::seed_from_u64(0);

        for _ in 0..100 {
            rng_core.fill_bytes(&mut content);

            let reader = Cursor::new(&content);

            let created = Blob::create_from_reader(reader).unwrap();

            let mut saved = Vec::new();
            created.save_to_writer(&mut saved).unwrap();

            let saved_cursor = Cursor::new(&saved);
            let read = Blob::read_from_reader(saved_cursor).unwrap();

            let final_content = read.content;

            assert_eq!(content, final_content);
        }
    }
}
