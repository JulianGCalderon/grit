use std::{
    env,
    fs::create_dir_all,
    io,
    path::{Path, PathBuf},
};

use thiserror::Error;

use crate::object::Blob;

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

pub fn blob(file: &Path) -> GitResult<String> {
    let git_dir = get_git_dir();

    let blob = Blob::create(file)?;
    let blob_id = blob.id().clone();
    let blob_path = git_dir.join(format!("objects/{}/{}", &blob_id[..2], &blob_id[2..]));
    if let Some(base) = blob_path.parent() {
        create_dir_all(base)?;
    };
    blob.save(blob_path)?;
    Ok(blob_id)
}
