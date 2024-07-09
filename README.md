# Grit

A Git implementation in Rust. Aims to be 100% compatible. I developed this purely for learning purposes and doesn't propose any advatange over using Git, not even in speed. I want to learn in detail the internals of Git and how the information is serialized. I do not intend to make a full Git rewrite, just enough to have a usable version control system

## Getting Started

### Dependencies

- Rust
- [Just](https://github.com/casey/just) (optional)

### Usage

To build the project, run:
```bash
just build
```

To test the project, run:
```bash
just test
```

To get a list of all recipes, run:
```bash
$ just -l
Available recipes:
    build  # build the project
    check  # check format and clippy
    format # run cargo fmt
    test   # run tests
```

By default, `.grit` is used as the default repository directory, it can be modified with by setting the environment variable `GRIT_DIR`

## Showcase

A short demostration on how to commit changes using plumbing commands:

```bash
$ grit init
$ echo hello > a.txt
$ grit hash-object a.txt
ce013625030ba8dba906f756967f9e9ca394464a
$ grit cat-file ce013625030ba8dba906f756967f9e9ca394464a
hello
$ grit update-index a.txt
$ grit write-tree
2e81171448eb9f2ee3821e3d447aa6b2fe3ddba1
$ grit commit-tree 2e81171448eb9f2ee3821e3d447aa6b2fe3ddba1 -m "First commit!"
e149ca0a1896643faa78966b668a1adb560c3853
$ echo goodbye > b.txt
$ grit update-index b.txt
$ grit write-tree
addfd494a22b9381eab528c16bd149548de3ea6f
$ grit commit-tree addfd494a22b9381eab528c16bd149548de3ea6f -p e149ca0a1896643faa78966b668a1adb560c3853 -m "Second commit!"
1d9c99de5449e4ab41030ea697b44c2f5dd55395
$ grit update-ref 1d9c99de5449e4ab41030ea697b44c2f5dd55395
```

As this implementation is Git-compatible, we can use `git` commands to inspect the repository state:

```bash
$ git status
On branch master
nothing to commit, working tree clean

$ git log --oneline
1d9c99d (HEAD -> master) Second commit!
e149ca0 First commit!

$ git log -1
commit 1d9c99de5449e4ab41030ea697b44c2f5dd55395 (HEAD -> master)
Author: John Doe <johndoe@mail.com>
Date:   Tue Jul 9 12:44:47 2024 -0300

    Second commit!

$ git diff HEAD~
diff --git a/b.txt b/b.txt
new file mode 100644
index 0000000..dd7e1c6
--- /dev/null
+++ b/b.txt
@@ -0,0 +1 @@
+goodbye

$ git ls-files --stage
100644 ce013625030ba8dba906f756967f9e9ca394464a 0	a.txt
100644 dd7e1c6f0fefe118f0b63d9f10908c460aa317a6 0	b.txt
```

## Roadmap

The roadmap is incomplete as I am not yet familiar with all git internal commands. I will updated it as I develop them.

### Plumbing

- [x] Hash Object
- [x] Cat File
- [x] Update Index
- [x] Write Tree
- [x] Commit Tree
- [x] Update Ref

### Porcelain

- [x] Init
- [ ] Add
- [ ] Commit
- [ ] Branch
- [ ] Checkout
- [ ] Merge
- [ ] Clone
