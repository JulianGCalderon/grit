use std::{
    env,
    fs::{create_dir_all, File},
    io::{self, Seek},
    path::{Path, PathBuf},
};

use thiserror::Error;

use crate::object::{Blob, Oid};

#[derive(Error, Debug)]
pub enum GitError {
    #[error(transparent)]
    IO(#[from] io::Error),

    #[error("invalid object id")]
    InvalidOid,
}

pub type GitResult<T> = Result<T, GitError>;

const GIT_DIR: &str = ".grit";
const GIT_DIR_ENV: &str = "GRIT_DIR";

const OBJECT_PREFIX_LENGTH: usize = 2;

pub const DEFAULT_BRANCH: &str = "master";
pub const DEFAULT_CONTENT: &str = "\
            [core]\n\
            \trepositoryformatversion = 0\n\
            \tfilemode = true\n\
            \tbare = false\n\
            \tlogallrefupdates\n";

pub fn get_git_dir() -> PathBuf {
    let git_dir = env::var(GIT_DIR_ENV);
    let git_dir = git_dir.as_deref().unwrap_or(GIT_DIR);
    PathBuf::from(git_dir)
}

pub fn get_object_path(git_dir: &Path, oid: &Oid) -> PathBuf {
    git_dir.join(format!(
        "objects/{}/{}",
        &oid[..OBJECT_PREFIX_LENGTH],
        &oid[OBJECT_PREFIX_LENGTH..]
    ))
}

pub fn get_reference_path(git_dir: &Path, name: &str) -> PathBuf {
    git_dir.join("refs/heads").join(name)
}

pub fn create_object_path(git_dir: &Path, oid: &Oid) -> GitResult<PathBuf> {
    let object_path = get_object_path(git_dir, oid);
    if let Some(base) = object_path.parent() {
        create_dir_all(base)?;
    };
    Ok(object_path)
}

pub fn blob(path: &Path) -> GitResult<Oid> {
    let git_dir = get_git_dir();

    let mut file = File::open(path)?;
    let length = file.metadata()?.len() as usize;

    let blob_id = Blob::hash(&mut file, length)?;

    let blob_path = get_object_path(&git_dir, &blob_id);
    if let Some(base) = blob_path.parent() {
        create_dir_all(base)?;
    };
    let blob_file = File::create(blob_path)?;

    file.seek(io::SeekFrom::Start(0))?;
    Blob::serialize(file, blob_file, length)?;

    Ok(blob_id)
}
