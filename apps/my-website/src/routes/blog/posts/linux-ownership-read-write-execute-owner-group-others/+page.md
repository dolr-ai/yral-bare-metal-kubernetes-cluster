---
title: "File Ownership in Linux: How read, write and execute works for files and directories"
description: "How file and directory ownership in Linux works in Linux with explanations on changing read, write and execute permissions using chmod and changing owners using chown"
author: "Saikat Das"
tags:
  - linux
  - directory
  - file
  - read
  - write
  - execute
  - owner
  - group
  - others
  - bash
  - command-line
  - shell
  - terminal
date: "2020-03-29"
icon: "bash.png"
---

As we've pointed out before,

> Everything in Linux is a file.

## Users

These files have permissions and users associated with them. There are three kinds of users in the Linux security model:

- `user` - these are individual users who can own files and directories and has control over its access
- `group` - individual users can belong to a group consisting of one or more users who are given access to files and directories by their owners
- `others` - apart from a group, an owner may also grant some set of access rights to everybody, referred to as the world

## Permissions

In Linux, access rights to files and directories are defined in terms of read access, write access and execution access.

The three kinds are:

| Permission | File                                                                                                                                                                                            | Directory                                                                                               |
| ---------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------- |
| `r`        | Allows a file to be opened and read                                                                                                                                                             | Allows a directory's contents to be listed if the execute attribute is also set                         |
| `w`        | Allows a file to be written to or truncated; however, this attribute does not allow files to be renamed or deleted. The ability to delete or rename files is determined by directory attributes | Allows files within a directory to be created, deleted and renamed if the execute attribute is also set |
| `x`        | Allows a file to be treated as a program and executed. Program files written in scripting languages must also be set as readable to executed                                                    | Allows a directory to be entered using `cd`                                                             |

## `ls` Output

If we look at an example output of the `ls` command we see,

```bash
-rwxrwxr-- 1 saikat saikat    0 Mar 29 19:20 dummy.sh
```

The first _10_ characters of the listing are the **file attributes**. The first character is the **file type**. Common file types are:

- `-` - a regular file
- `d` - a directory
- `l` - a symbolic link

Remaining characters of the file attributes are called the **file mode**. They represent the read, write and execute permissions for the file's owner, the group owner and everybody else. In this case, they are:

- `rwx` - file owner has read, write and execute permission
- `rwx` - file group owner has read, write and execute permission
- `r--` - others only have read permission

## Changing permissions

We use `chmod` to can change permissions of a particular file or directory in Linux. There are _two_ ways of specifying mode changes:

- octal number representation
- symbolic representation

I use the **symbolic** method because it's more visual and **only** changes the permissions that you wish to change and **not the entire mode**. You also don't need to perform complicated binary to octal conversion to figure out the mode to pass.

Symbolic notation has **3** parts:

- who the change will affect. Possible values are:
  - `u` - short for user. Refers to the file or directory owner
  - `g` - short for group owner
  - `o` - short for others
  - `a` - short for all
- which operation will be performed. Possible values are:
  - `+` - indicates that a permission needs to be added
  - `-` - indicates that a permission needs to be taken away
  - `=` - indicates that the specified permissions need to be set and all others are to be removed
- what permission will be set. Possible values are:
  - `r` - read permission
  - `w` - write permission
  - `x` - execute permission

Example:

```bash
chmod u=rwx,g=rx,o=r myfile
```

### `chmod` modes examples

- `u+x` - add execute permission for the owner
- `u-x` - remove execute permission from the owner
- `a+x` - add execute permission for the owner, group and others
- `o-rw` - remove the read and write permissions from anyone besides the owner and group owner
- `go=rw` - set the group owner and anyone besides the owner to have read and write permissions. If either had execute permission, that is removed
- `u+x,go=rx` - add execute permission for the owner and set the permissions for the group and others to read and execute

## Changing Ownership

Ownership of files and directories are changed with the `chown` command. Syntax is:

```bash
chown [owner][:[group]] file...
```

`chown` can change the file owner and/or the file group owner depending on the first argument of the command.

### `chown` argument examples

- `saikat` - changes the ownership of the file from its current owner to user _saikat_
- `saikat:users` - changes the ownership of the file from its current owner to user _saikat_ and group owner to group _users_
- `:admins` - changes the group owner to the group _admins_. The file owner is unchanged
- `saikat:` - changes the file owner from the current owner to user _saikat_ and changes the group owner to the login group of user _saikat_
