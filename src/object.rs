use std::{fmt::Display, ops::Deref};

use crate::repository::{GitError, GitResult};

mod blob;
mod tree;

pub use blob::Blob;
pub use tree::{Tree, TreeEntry};

pub const OID_HEX_LEN: usize = 20;

#[derive(Clone, Hash, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub struct Oid(String);

impl Oid {
    pub fn new(id: impl Into<String>) -> GitResult<Self> {
        let hex_id = id.into();
        let decoded_len =
            base16ct::decoded_len(hex_id.as_bytes()).map_err(|_| GitError::InvalidOid)?;
        if decoded_len != OID_HEX_LEN {
            Err(GitError::InvalidOid)
        } else {
            Ok(Self(hex_id))
        }
    }

    pub fn to_raw_bytes(&self) -> [u8; OID_HEX_LEN] {
        let raw_id = base16ct::lower::decode_vec(self.0.as_bytes()).expect("should never fail");
        raw_id.try_into().expect("should never fail")
    }

    pub fn from_raw_bytes(raw_bytes: [u8; OID_HEX_LEN]) -> Self {
        let hex_id = base16ct::lower::encode_string(&raw_bytes);
        Self(hex_id)
    }
}

impl AsRef<str> for Oid {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl Deref for Oid {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        self.as_ref()
    }
}

impl Display for Oid {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}
