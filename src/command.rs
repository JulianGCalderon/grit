use std::{
    env,
    fs::{create_dir_all, remove_dir_all, write, File},
    io::{self, stdout, BufRead as _, BufReader, Read as _, Seek as _, Write as _},
    os::unix::{ffi::OsStrExt as _, fs::MetadataExt as _},
    path::Path,
};

use flate2::Compression;
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

    let mut encoder = flate2::write::ZlibEncoder::new(&mut object_file, Compression::default());

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

pub fn cat_file(hash: &str) -> GitResult<()> {
    let object_path = {
        let git_dir = env::var(GIT_DIR_ENV);
        let git_dir = git_dir.as_deref().unwrap_or(GIT_DIR);
        let git_dir = Path::new(git_dir);
        git_dir.join(&format!("objects/{}/{}", &hash[..2], &hash[2..]))
    };

    let object_file = File::open(object_path)?;

    let mut decoder = BufReader::new(flate2::read::ZlibDecoder::new(&object_file));

    let mut object_header = Vec::new();
    let _ = decoder.read_until(0, &mut object_header)?;
    // todo: handle object header

    io::copy(&mut decoder, &mut stdout())?;

    Ok(())
}

pub fn update_index(file: &Path) -> GitResult<()> {
    let index_path = {
        let git_dir = env::var(GIT_DIR_ENV);
        let git_dir = git_dir.as_deref().unwrap_or(GIT_DIR);
        let git_dir = Path::new(git_dir);
        git_dir.join("index")
    };

    let metadata = file.metadata()?;

    // todo: handle appending
    let mut index_file = File::options()
        .create(true)
        .read(true)
        .write(true)
        .truncate(true)
        .open(index_path)?;

    index_file.write_all("DIRC".as_bytes())?;
    index_file.write_all(&[0, 0, 0, 2])?;
    index_file.write_all(&[0, 0, 0, 1])?;
    index_file.write_all(&(metadata.ctime() as i32).to_be_bytes())?;
    index_file.write_all(&(metadata.ctime_nsec() as i32).to_be_bytes())?;
    index_file.write_all(&(metadata.mtime() as i32).to_be_bytes())?;
    index_file.write_all(&(metadata.mtime_nsec() as i32).to_be_bytes())?;
    index_file.write_all(&(metadata.dev() as u32).to_be_bytes())?;
    index_file.write_all(&(metadata.ino() as u32).to_be_bytes())?;
    index_file.write_all(&metadata.mode().to_be_bytes())?;
    index_file.write_all(&(metadata.uid()).to_be_bytes())?;
    index_file.write_all(&(metadata.gid()).to_be_bytes())?;
    index_file.write_all(&(metadata.size() as u32).to_be_bytes())?;

    let mut object_file = File::open(file)?;

    let mut hasher = Sha1::new();

    let file_size = object_file.metadata()?.len();
    let header = format!("blob {file_size}\0");

    hasher.update(&header);
    let read_file_size = io::copy(&mut object_file, &mut hasher)?;

    assert_eq!(
        file_size, read_file_size,
        "metadata file size is different from real file size"
    );

    let user_hash = hasher.finalize();

    index_file.write_all(&user_hash)?;

    // todo: canonicalize as relative
    let canonicalized_name = file.as_os_str().as_bytes();

    let assume_valid = 0 as u16;
    let extended_flag = 0 as u16;
    let stage = 0.min(0b11) as u16;
    let name_length = canonicalized_name.len().min(0xFFF) as u16;

    let flags = name_length + (stage << 12) + (extended_flag << 14) + (assume_valid << 15);

    index_file.write_all(&flags.to_be_bytes())?;

    index_file.write_all(canonicalized_name)?;

    index_file.flush()?;
    let size = (index_file.metadata()?.size() + 20) % 8;

    let padding = vec![0; (8 - size) as usize];

    index_file.write_all(&padding)?;

    let mut hasher = Sha1::new();

    index_file.flush()?;
    index_file.seek(io::SeekFrom::Start(0))?;
    let _read_file_size = io::copy(&mut index_file, &mut hasher)?;

    let index_hash = hasher.finalize();

    index_file.write_all(&index_hash)?;

    {
        let object_path = {
            let git_dir = env::var(GIT_DIR_ENV);
            let git_dir = git_dir.as_deref().unwrap_or(GIT_DIR);
            let git_dir = Path::new(git_dir);
            let hex_hash = base16ct::lower::encode_string(&user_hash);
            git_dir.join(&format!("objects/{}/{}", &hex_hash[..2], &hex_hash[2..]))
        };

        if let Some(base) = object_path.parent() {
            create_dir_all(base)?;
        };

        let mut object_file = File::create(object_path)?;

        let mut user_file = File::open(file)?;

        let mut encoder = flate2::write::ZlibEncoder::new(&mut object_file, Compression::default());

        encoder.write(header.as_bytes())?;

        let write_file_size = io::copy(&mut user_file, &mut encoder)?;
        assert_eq!(
            read_file_size, write_file_size,
            "read file size is different from write file size"
        );

        encoder.finish()?;
    }

    Ok(())
}

pub fn write_tree() -> GitResult<()> {
    let index_path = {
        let git_dir = env::var(GIT_DIR_ENV);
        let git_dir = git_dir.as_deref().unwrap_or(GIT_DIR);
        let git_dir = Path::new(git_dir);
        git_dir.join("index")
    };

    let mut index_file = BufReader::new(File::open(index_path)?);

    let mut index_header = vec![0; 12];
    index_file.read_exact(&mut index_header)?;

    let mut entry_header = vec![0; 24];
    index_file.read_exact(&mut entry_header)?;
    let mut entry_mode = vec![0; 4];
    index_file.read_exact(&mut entry_mode)?;
    let mut entry_header = vec![0; 12];
    index_file.read_exact(&mut entry_header)?;
    let mut entry_hash = vec![0; 20];
    index_file.read_exact(&mut entry_hash)?;
    let mut entry_flags = vec![0; 2];
    index_file.read_exact(&mut entry_flags)?;
    let mut entry_name = Vec::new();
    let name_length = index_file.read_until(b'\0', &mut entry_name)?;
    entry_name.pop();

    let padding = 8 - (((name_length) + 12 + 2) % 8);
    let mut padding_bytes = vec![0; padding];
    index_file.read_exact(&mut padding_bytes)?;

    let mut tree_entries = Vec::new();

    let entry_mode = entry_mode.try_into().unwrap();
    let entry_mode = u32::from_be_bytes(entry_mode);

    let file_type_1 = mask_and_cast(entry_mode, 0o100000);
    let file_type_2 = mask_and_cast(entry_mode, 0o070000);
    let special = mask_and_cast(entry_mode, 0o7000);
    let owner = mask_and_cast(entry_mode, 0o0700);
    let group = mask_and_cast(entry_mode, 0o0070);
    let others = mask_and_cast(entry_mode, 0o0007);

    tree_entries.push(file_type_1);
    tree_entries.push(file_type_2);
    tree_entries.push(special);
    tree_entries.push(owner);
    tree_entries.push(group);
    tree_entries.push(others);
    tree_entries.push(b' ');
    tree_entries.append(&mut entry_name);
    tree_entries.push(b'\0');
    tree_entries.append(&mut entry_hash);

    let mut hasher = Sha1::new();

    let file_size = tree_entries.len();
    let header = format!("tree {file_size}\0");

    hasher.update(&header);
    hasher.update(&tree_entries);

    let tree_hash = hasher.finalize();

    let tree_hex_hash = base16ct::lower::encode_string(&tree_hash);
    println!("{tree_hex_hash}");

    let tree_path = {
        let git_dir = env::var(GIT_DIR_ENV);
        let git_dir = git_dir.as_deref().unwrap_or(GIT_DIR);
        let git_dir = Path::new(git_dir);
        git_dir.join(format!(
            "objects/{}/{}",
            &tree_hex_hash[..2],
            &tree_hex_hash[2..]
        ))
    };

    if let Some(base) = tree_path.parent() {
        create_dir_all(base)?;
    };

    let mut tree_file = File::create(tree_path)?;

    let mut encoder = flate2::write::ZlibEncoder::new(&mut tree_file, Compression::default());

    encoder.write_all(header.as_bytes())?;
    encoder.write_all(&tree_entries)?;

    Ok(())
}

/// masks the mode bits, discards trailing zeroes,
/// and converts the result to a numerical character
fn mask_and_cast(mode: u32, mask: u32) -> u8 {
    let masked = mode & mask;

    let shifted = masked >> mask.trailing_zeros();

    shifted as u8 + b'0'
}
