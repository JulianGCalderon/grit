# Grit

## Showcase

```bash
$ grit init
$ echo hello > a.txt
$ grit update-index a.txt
$ grit write-tree
2e81171448eb9f2ee3821e3d447aa6b2fe3ddba1
$ grit commit-tree 2e81171448eb9f2ee3821e3d447aa6b2fe3ddba1 -m "First commit!"
4847aab59d41af650ad4aaf72e1444799427b232
$ echo goodbye > b.txt
$ grit update-index b.txt
$ grit write-tree
addfd494a22b9381eab528c16bd149548de3ea6f
$ grit commit-tree addfd494a22b9381eab528c16bd149548de3ea6f -p 4847aab59d41af650ad4aaf72e1444799427b232 -m "Second commit!"
cabf8a1d63bcfa77a9f3944b50c037b5b7653d96
$ grit update-ref cabf8a1d63bcfa77a9f3944b50c037b5b7653d96
$ git log
commit cabf8a1d63bcfa77a9f3944b50c037b5b7653d96 (HEAD -> master)
Author: John Doe <johndoe@mail.com>
Date:   Tue Jul 9 03:16:59 2024 -0300

    Second commit!

commit 4847aab59d41af650ad4aaf72e1444799427b232
Author: John Doe <johndoe@mail.com>
Date:   Tue Jul 9 03:16:26 2024 -0300

    First commit!
$ git diff 4847aab59d41af650ad4aaf72e1444799427b232
diff --git a/b.txt b/b.txt
new file mode 100644
index 0000000..dd7e1c6
--- /dev/null
+++ b/b.txt
@@ -0,0 +1 @@
+goodbye
```
