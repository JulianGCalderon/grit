use std::{
    fs::{self, create_dir, create_dir_all, write, File},
    io,
    path::PathBuf,
};

use crate::{
    index::{Index, IndexEntry},
    object::{Blob, Commit, Oid, Tree, TreeEntry},
    repository::{
        blob, create_object_path, get_git_dir, get_object_path, get_reference_path, GitResult,
        DEFAULT_BRANCH, DEFAULT_CONTENT,
    },
};

pub fn init() -> GitResult<()> {
    let git_dir = get_git_dir();

    create_dir_all(&git_dir)?;

    let head = git_dir.join("HEAD");
    if !head.exists() {
        write(
            head,
            format!(
                "ref: {}",
                get_reference_path(&git_dir, DEFAULT_BRANCH)
                    .into_os_string()
                    .into_string()
                    .expect("references are always utf8")
            )
            .as_bytes(),
        )?;
    }

    let config = git_dir.join("config");
    if !config.exists() {
        write(config, DEFAULT_CONTENT.as_bytes())?;
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

pub fn hash_object(file: PathBuf) -> GitResult<()> {
    let blob_id = blob(&file)?;

    println!("{blob_id}");

    Ok(())
}

pub fn cat_file(id: String) -> GitResult<()> {
    let oid = Oid::new(id)?;

    let git_dir = get_git_dir();
    let blob_path = get_object_path(&git_dir, &oid);
    let blob_file = File::open(blob_path)?;

    Blob::deserialize(blob_file, io::stdout())?;

    Ok(())
}

pub fn update_index(file: PathBuf) -> GitResult<()> {
    let git_dir = get_git_dir();

    let index_path = git_dir.join("index");
    let mut index = Index::deserialize_from_path(&index_path).unwrap_or_default();

    let blob_id = blob(&file)?;

    let entry = {
        let metadata = fs::metadata(&file)?;
        let name = file
            .into_os_string()
            .into_string()
            .expect("filename is not utf8");
        IndexEntry::new(metadata, blob_id, false, 0, name)?
    };

    index.push(entry);

    index.serialize_to_path(index_path)?;

    Ok(())
}

pub fn write_tree() -> GitResult<()> {
    let git_dir = get_git_dir();

    let index_path = git_dir.join("index");
    let index = Index::deserialize_from_path(index_path)?;

    let tree = Tree::new(
        index
            .entries()
            .iter()
            .map(|index_entry| {
                TreeEntry::new(
                    index_entry.mode(),
                    index_entry.name().to_string(),
                    index_entry.oid().clone(),
                )
                .expect("index entries are always valid")
            })
            .collect(),
    );

    let tree_id = tree.hash();
    let tree_path = create_object_path(&git_dir, &tree_id)?;
    let tree_file = File::create(tree_path)?;

    tree.serialize(tree_file)?;

    println!("{}", tree_id);

    Ok(())
}

pub fn commit_tree(tree_id: String, parent: Option<String>, message: String) -> GitResult<()> {
    let git_dir = get_git_dir();

    let mut parents = Vec::new();

    if let Some(parent) = parent {
        parents.push(Oid::new(parent)?)
    };

    let commit = Commit::new(
        parents,
        Oid::new(tree_id)?,
        message.to_string(),
        "John Doe".to_string(),
        "johndoe@mail.com".to_string(),
        "John Doe".to_string(),
        "johndoe@mail.com".to_string(),
    )?;

    let commit_id = commit.hash();
    let commit_path = create_object_path(&git_dir, &commit_id)?;
    let commit_file = File::create(commit_path)?;

    commit.serialize(commit_file)?;

    println!("{}", commit_id);

    Ok(())
}
