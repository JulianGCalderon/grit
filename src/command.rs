use std::{
    env,
    fs::{create_dir_all, remove_dir_all, write},
    io,
    path::PathBuf,
};

use thiserror::Error;

const GIT_DIR: &str = ".grit";
const GIT_DIR_ENV: &str = "GRIT_DIR";

#[derive(Error, Debug)]
pub enum GitError {
    #[error(transparent)]
    IO(#[from] io::Error),
}

pub type GitResult<T> = Result<T, GitError>;

pub fn init() -> GitResult<()> {
    let git_dir = {
        let git_dir = env::var(GIT_DIR_ENV);
        let git_dir = git_dir.as_deref().unwrap_or(GIT_DIR);
        PathBuf::from(git_dir)
    };

    let _ = remove_dir_all(&git_dir);
    create_dir_all(&git_dir)?;

    let head = git_dir.join("HEAD");
    if !head.exists() {
        write(head, "ref: refs/heads/master\n")?;
    }

    let config = git_dir.join("config");
    if !config.exists() {
        let contents = "\
            [core]\n\
            \trepositoryformatversion = 0\n\
            \tfilemode = true\n\
            \tbare = false\n\
            \tlogallrefupdates\n";

        write(config, contents)?;
    }

    let branches = git_dir.join("branches");
    let hooks = git_dir.join("hooks");
    let info = git_dir.join("info");
    create_dir_all(branches)?;
    create_dir_all(hooks)?;
    create_dir_all(info)?;

    let objects = git_dir.join("objects");
    let objects_info = objects.join("objects_info");
    let objects_pack = objects.join("objects_pack");
    create_dir_all(objects_info)?;
    create_dir_all(objects_pack)?;

    let refs = git_dir.join("refs");
    let refs_heads = refs.join("heads");
    let refs_tags = refs.join("tags");
    create_dir_all(refs_heads)?;
    create_dir_all(refs_tags)?;

    Ok(())
}
