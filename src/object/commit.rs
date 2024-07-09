use std::io::Write;

use flate2::{write::ZlibEncoder as ZlibWriteEncoder, Compression};
use sha1::{Digest, Sha1};

use crate::repository::GitResult;

use super::Oid;

pub struct Commit {
    parents: Vec<Oid>,
    tree_id: Oid,
    message: String,
    author: String,
    author_email: String,
    commiter: String,
    commiter_email: String,
}

impl Commit {
    pub fn new(
        parents: Vec<Oid>,
        tree_id: Oid,
        mut message: String,
        author: String,
        author_email: String,
        commiter: String,
        commiter_email: String,
    ) -> GitResult<Self> {
        if message
            .as_bytes()
            .last()
            .map(|&last| last != b'\n')
            .unwrap_or_default()
        {
            message.push('\n')
        }

        Ok(Self {
            parents,
            tree_id,
            message,
            author,
            author_email,
            commiter,
            commiter_email,
        })
    }

    pub fn hash(&self) -> Oid {
        let mut hasher = Sha1::new();
        let header = self.header();

        hasher.update(&header);
        self.serialize(&mut hasher)
            .expect("writing to hasher cannot fail");

        let raw_id = hasher.finalize().into();

        Oid::from_raw_bytes(raw_id)
    }
    pub fn serialize<W: Write>(&self, writer: W) -> GitResult<()> {
        let mut encoder = ZlibWriteEncoder::new(writer, Compression::default());

        encoder.write_all(&self.header())?;

        let tree_line = format!("tree {}\n", self.tree_id);
        encoder.write_all(tree_line.as_bytes())?;

        let timestamp = chrono::Local::now().format("%s %z");

        let author_line = format!(
            "author {} <{}> {}\n",
            self.author, self.author_email, timestamp
        );
        encoder.write_all(author_line.as_bytes())?;

        let commiter_line = format!(
            "committer {} <{}> {}\n\n",
            self.commiter, self.commiter_email, timestamp
        );
        encoder.write_all(commiter_line.as_bytes())?;

        encoder.write_all(self.message.as_bytes())?;

        Ok(())
    }

    pub fn header(&self) -> Vec<u8> {
        format!("commit {}\0", self.size()).into_bytes()
    }

    fn size(&self) -> usize {
        // 106 bytes are fixed, the rest is variable
        106 + self.author.as_bytes().len()
            + self.author_email.as_bytes().len()
            + self.commiter.as_bytes().len()
            + self.commiter_email.as_bytes().len()
            + self.message.as_bytes().len()
    }
}

#[cfg(test)]
mod tests {
    use flate2::write::ZlibDecoder as ZlibWriteDecoder;
    use pretty_assertions_sorted::assert_eq;
    use std::io;

    use super::*;

    #[test]
    pub fn size_calculation_is_correct() {
        let commit = Commit::new(
            vec![
                Oid::new("554b0c91f951764bb11f1db849685d95b2c7a48f").unwrap(),
                Oid::new("bedc28ca5099946b354104a3c6cc90ec20dbcaec").unwrap(),
            ],
            Oid::new("f0133c7517d34d37f8dca8c8444c6a9cdd7e4cdc").unwrap(),
            "message".to_string(),
            "John Doe".to_string(),
            "johndoe@mail.com".to_string(),
            "John Doe".to_string(),
            "johndoe@mail.com".to_string(),
        )
        .unwrap();

        let mut serialized = Vec::new();
        commit.serialize(&mut serialized).unwrap();

        let mut decoded = Vec::new();
        let mut decoder = ZlibWriteDecoder::new(&mut decoded);
        io::copy(&mut serialized.as_slice(), &mut decoder).unwrap();
        decoder.finish().unwrap();

        let header = commit.header();

        assert_eq!(decoded.len() - header.len(), commit.size())
    }
}
