use std::io::Write;

use flate2::{write::ZlibEncoder as ZlibWriteEncoder, Compression};
use sha1::{Digest, Sha1};

use crate::repository::GitResult;

use super::Oid;

pub struct Commit {
    tree_id: Oid,
    message: String,
    author: String,
    author_email: String,
    commiter: String,
    commiter_email: String,
}

impl Commit {
    pub fn new(
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
        // 106 bytes are fixed, the rest is variable
        let size = 106
            + self.author.as_bytes().len()
            + self.author_email.as_bytes().len()
            + self.commiter.as_bytes().len()
            + self.commiter_email.as_bytes().len()
            + self.message.as_bytes().len();
        format!("commit {size}\0").into_bytes()
    }
}
