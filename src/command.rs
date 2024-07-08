use std::{
    fs::{self, create_dir, create_dir_all, remove_dir_all, write, File},
    io::{self, Write},
    os::unix::fs::MetadataExt as _,
    path::Path,
};

use flate2::Compression;
use sha1::{Digest, Sha1};

use crate::{
    index::{Index, IndexEntry},
    object::{Blob, Oid, Tree, TreeEntry},
    repository::{blob, get_git_dir, get_object_path, GitResult},
};

pub fn init() -> GitResult<()> {
    let git_dir = get_git_dir();

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
    create_dir(branches)?;
    create_dir(hooks)?;
    create_dir(info)?;

    let objects = git_dir.join("objects");
    let objects_info = objects.join("objects_info");
    let objects_pack = objects.join("objects_pack");
    create_dir_all(objects_info)?;
    create_dir(objects_pack)?;

    let refs = git_dir.join("refs");
    let refs_heads = refs.join("heads");
    let refs_tags = refs.join("tags");
    create_dir_all(refs_heads)?;
    create_dir(refs_tags)?;

    Ok(())
}

pub fn hash_object(file: &Path) -> GitResult<()> {
    let blob_id = blob(file)?;

    println!("{blob_id}");

    Ok(())
}

pub fn cat_file(id: &str) -> GitResult<()> {
    let oid = Oid::new(id)?;

    let git_dir = get_git_dir();
    let blob_path = get_object_path(&git_dir, &oid);
    let blob_file = File::open(blob_path)?;

    Blob::deserialize(blob_file, io::stdout())?;

    Ok(())
}

pub fn update_index(file: &Path) -> GitResult<()> {
    let git_dir = get_git_dir();

    let index_path = git_dir.join("index");
    let mut index = Index::deserialize_from_path(&index_path).unwrap_or_default();

    let blob_id = blob(file)?;

    let entry = {
        let metadata = fs::metadata(file)?;
        let name = file.to_str().expect("filename is not utf8").to_string();
        IndexEntry {
            ctime: metadata.ctime() as i32,
            ctime_nsec: metadata.ctime_nsec() as i32,
            mtime: metadata.mtime() as i32,
            mtime_nsec: metadata.mtime_nsec() as i32,
            dev: metadata.dev() as u32,
            ino: metadata.ino() as u32,
            mode: metadata.mode() as u32,
            uid: metadata.uid() as u32,
            gid: metadata.gid() as u32,
            size: metadata.size() as u32,
            oid: blob_id.clone(),
            assume_valid: false,
            stage: 0,
            name,
        }
    };

    index.entries.insert(blob_id, entry);

    index.serialize_to_path(index_path)?;

    Ok(())
}

pub fn write_tree() -> GitResult<()> {
    let git_dir = get_git_dir();

    let index_path = git_dir.join("index");
    let index = Index::deserialize_from_path(index_path)?;

    let mut index_entries: Vec<IndexEntry> = index.entries.clone().into_values().collect();
    index_entries.sort();

    let tree = Tree {
        entries: index_entries
            .iter()
            .map(|index_entry| TreeEntry {
                mode: index_entry.mode,
                name: index_entry.name.clone(),
                oid: index_entry.oid.clone(),
            })
            .collect(),
    };

    let tree_id = tree.hash();
    let tree_path = get_object_path(&git_dir, &tree_id);
    let tree_file = File::create(tree_path)?;

    tree.serialize(tree_file)?;

    println!("{}", tree_id);

    Ok(())
}

pub fn commit_tree(hash: &str, message: Option<&str>) -> GitResult<()> {
    let git_dir = get_git_dir();

    let mut tree_entries = Vec::new();

    tree_entries.write_all("tree ".as_bytes())?;
    tree_entries.write_all(hash.as_bytes())?;
    tree_entries.push(b'\n');
    tree_entries.write_all("author author <author@mail.com> ".as_bytes())?;

    let timestamp = chrono::Local::now();

    tree_entries.write_all(timestamp.format("%s %z\n").to_string().as_bytes())?;
    tree_entries.write_all("commiter commiter <commiter@mail.com> ".as_bytes())?;
    tree_entries.write_all(timestamp.format("%s %z\n\n").to_string().as_bytes())?;

    if let Some(message) = message {
        tree_entries.write_all(message.as_bytes())?;
        tree_entries.push(b'\n');
    }

    let mut hasher = Sha1::new();

    let file_size = tree_entries.len();
    let header = format!("commit {file_size}\0");

    hasher.update(&header);
    hasher.update(&tree_entries);

    let hash = hasher.finalize();

    let hex_hash = base16ct::lower::encode_string(&hash);

    let commit_path = git_dir.join(format!("objects/{}/{}", &hex_hash[..2], &hex_hash[2..]));

    let parent = commit_path.parent();
    if let Some(parent) = parent {
        create_dir_all(parent)?;
    }

    let mut commit_file = File::create(commit_path)?;
    let mut encoder = flate2::write::ZlibEncoder::new(&mut commit_file, Compression::default());
    encoder.write_all(header.as_bytes())?;
    encoder.write_all(&tree_entries)?;

    let commit_path = git_dir.join(format!("refs/heads/master"));
    let mut commit_file = File::create(&commit_path)?;
    commit_file.write_all(hex_hash.as_bytes())?;
    commit_file.write_all(&[b'\n'])?;

    println!("{}", hex_hash);

    Ok(())
}
