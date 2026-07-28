---
title: "Links in Linux: Symbolic and Hard Linking"
description: "A summary of symbolic and hard links in Linux with guidance on when to use symolic links and when to use hard links"
author: "Saikat Das"
tags:
  - linux
  - link
  - symbolic-link
  - hard-link
  - file
  - bash
  - command-line
date: "2020-03-18"
icon: "bash.png"
---

We create links in Linux using the `ln` command. There are two kinds of links in Linux, namely,

- symbolic links
- hard links

The syntax for creating links is

```bash
ln -s file link
```

The `-s` creates **symbolic** links and can be excluded to create **hard** links

## Symbolic Links

Symbolic links are the modern way of creating links and were created to overcome the limitations of hard links. They work by creating a special type of file that contains a text pointer to the referenced file or directory. A file pointed to by a symbolic link and the symbolic link itself are largely indistinguishable from one another. If we write something to the symbolic link, the referenced file is written to. However, if we delete a symbolic link, only the link is deleted, not the file itself

## Hard Links

We should use symbolic links for our purposes of shortcuts and such. Hard links are the original Unix way of creating links. By default, every file has a single hard link that gives the file its name
