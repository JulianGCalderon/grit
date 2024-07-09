use std::{
    fs::{self, create_dir, create_dir_all, remove_dir_all, write, File},
    io::{self},
    os::unix::fs::MetadataExt as _,
    path::Path,
};

use crate::{
    index::{Index, IndexEntry},
    object::{Blob, Commit, Oid, Tree, TreeEntry},
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
    if let Some(base) = tree_path.parent() {
        create_dir_all(base)?;
    };
    let tree_file = File::create(tree_path)?;

    tree.serialize(tree_file)?;

    println!("{}", tree_id);

    Ok(())
}

pub fn commit_tree(tree_id: &str, message: &str) -> GitResult<()> {
    let git_dir = get_git_dir();

    let commit = Commit {
        tree_id: Oid::new(tree_id)?,
        message: message.to_string(),
        author: "John Doe".to_string(),
        author_email: "johndoe@mail.com".to_string(),
        commiter: "John Doe".to_string(),
        commiter_email: "johndoe@mail.com".to_string(),
    };

    let commit_id = commit.hash();
    let commit_path = get_object_path(&git_dir, &commit_id);
    if let Some(base) = commit_path.parent() {
        create_dir_all(base)?;
    };
    let commit_file = File::create(commit_path)?;

    commit.serialize(commit_file)?;

    println!("{}", commit_id);

    Ok(())
}
