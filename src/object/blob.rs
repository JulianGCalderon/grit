use std::io::{self, BufRead, BufReader, Read, Write};

use flate2::{
    read::ZlibDecoder as ZlibReadDecoder, write::ZlibEncoder as ZlibWriteEncoder, Compression,
};
use sha1::{Digest, Sha1};

use crate::repository::GitResult;

use super::Oid;

pub struct Blob;

impl Blob {
    pub fn hash<R: Read>(mut plain: R, length: usize) -> GitResult<Oid> {
        let mut hasher = Sha1::new();

        let header = Self::header(length);
        hasher.update(&header);

        io::copy(&mut plain, &mut hasher)?;

        let raw_id = hasher.finalize().into();
        let id = Oid::from_raw_bytes(raw_id);

        Ok(id)
    }

    pub fn serialize<R: Read, W: Write>(mut src: R, dst: W, length: usize) -> GitResult<()> {
        let mut encoder = ZlibWriteEncoder::new(dst, Compression::default());

        encoder.write_all(&Self::header(length))?;
        io::copy(&mut src, &mut encoder)?;

        Ok(())
    }

    pub fn deserialize<R: Read, W: Write>(src: R, mut dst: W) -> GitResult<()> {
        let mut decoder = BufReader::new(ZlibReadDecoder::new(src));

        let mut object_header = Vec::new();
        decoder.read_until(0, &mut object_header)?;

        io::copy(&mut decoder, &mut dst)?;

        Ok(())
    }

    pub fn header(size: usize) -> Vec<u8> {
        format!("blob {size}\0").bytes().collect()
    }
}

#[cfg(test)]
mod tests {
    use rand::{RngCore, SeedableRng};

    use super::*;

    #[test]
    pub fn can_serialize_and_deserialize() {
        let mut original = vec![0; 100];
        let mut rng_core = rand::rngs::StdRng::seed_from_u64(0);

        for _ in 0..100 {
            rng_core.fill_bytes(&mut original);

            let mut serialized = Vec::with_capacity(original.len());
            Blob::serialize(original.as_slice(), &mut serialized, original.len()).unwrap();

            let mut deserialized = Vec::with_capacity(original.len());
            Blob::deserialize(serialized.as_slice(), &mut deserialized).unwrap();

            assert_eq!(original, deserialized);
        }
    }
}
