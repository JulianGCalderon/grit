use std::{fs::create_dir_all, io};

use thiserror::Error;

const GIT_DIR: &str = ".grit";

#[derive(Error, Debug)]
pub enum GitError {
    #[error(transparent)]
    IO(#[from] io::Error),
}

pub type GitResult<T> = Result<T, GitError>;

pub fn init() -> GitResult<()> {
    create_dir_all(GIT_DIR)?;

    Ok(())
}
