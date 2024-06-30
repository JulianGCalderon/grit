use std::{
    env,
    fs::{create_dir_all, remove_dir_all, write, File},
    io::{self, Seek, Write},
    path::Path,
};

use flate2::{write::ZlibEncoder, Compression};
use sha1::{Digest, Sha1};
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
    let git_dir = env::var(GIT_DIR_ENV);
    let git_dir = git_dir.as_deref().unwrap_or(GIT_DIR);
    let git_dir = Path::new(git_dir);

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

pub fn hash_object(file: &Path) -> GitResult<()> {
    let mut file = File::open(file)?;

    let mut hasher = Sha1::new();

    let file_size = file.metadata()?.len();
    let header = format!("blob {file_size}\0");

    hasher.update(&header);
    let read_file_size = io::copy(&mut file, &mut hasher)?;

    assert_eq!(
        file_size, read_file_size,
        "metadata file size is different from real file size"
    );

    let hash = hasher.finalize();

    let hex_hash = base16ct::lower::encode_string(&hash);
    println!("{hex_hash}");

    let object_path = {
        let git_dir = env::var(GIT_DIR_ENV);
        let git_dir = git_dir.as_deref().unwrap_or(GIT_DIR);
        let git_dir = Path::new(git_dir);
        git_dir.join(&format!("objects/{}/{}", &hex_hash[..2], &hex_hash[2..]))
    };

    if let Some(base) = object_path.parent() {
        create_dir_all(base)?;
    };

    let mut object_file = File::create(object_path)?;

    let mut encoder = ZlibEncoder::new(&mut object_file, Compression::default());

    encoder.write(header.as_bytes())?;

    file.seek(io::SeekFrom::Start(0))?;
    let write_file_size = io::copy(&mut file, &mut encoder)?;
    assert_eq!(
        read_file_size, write_file_size,
        "read file size is different from write file size"
    );

    encoder.finish()?;

    Ok(())
}
