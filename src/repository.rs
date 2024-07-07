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
}

pub type GitResult<T> = Result<T, GitError>;

const GIT_DIR: &str = ".grit";
const GIT_DIR_ENV: &str = "GRIT_DIR";

pub fn get_git_dir() -> PathBuf {
    let git_dir = env::var(GIT_DIR_ENV);
    let git_dir = git_dir.as_deref().unwrap_or(GIT_DIR);
    PathBuf::from(git_dir)
}

pub fn blob(path: &Path) -> GitResult<Oid> {
    let git_dir = get_git_dir();

    let mut file = File::open(path)?;
    let length = file.metadata()?.len() as usize;

    let blob_id = Blob::hash(&mut file, length)?;
    let blob_id_string = blob_id.to_string();

    let blob_path = git_dir.join(format!(
        "objects/{}/{}",
        &blob_id_string[..2],
        &blob_id_string[2..]
    ));
    if let Some(base) = blob_path.parent() {
        create_dir_all(base)?;
    };
    let blob_file = File::create(blob_path)?;

    file.seek(io::SeekFrom::Start(0))?;

    Blob::serialize(file, blob_file, length)?;

    Ok(blob_id)
}
