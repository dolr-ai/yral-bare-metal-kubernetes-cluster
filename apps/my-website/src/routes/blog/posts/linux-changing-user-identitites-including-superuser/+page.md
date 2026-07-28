---
title: "User Identities in Linux: How to use the sudo and su commands"
description: "Using sudo and su effectively to gain superuser abilities on a per command basis or to login to the shell as a different user"
author: "Saikat Das"
tags:
  - linux
  - users
  - identities
  - sudo
  - su
  - bash
  - command-line
  - shell
  - terminal
date: "2020-03-31"
icon: "bash.png"
---

Often we want to gain superuser privileges to carry out some administrative task, but it is also possible to impersonate another regular user. We can do so using the `sudo` and `su` commands as explained below.

## `sudo` command

The `sudo` command has some advantages over the `su` command. They are:

- The administrator can configure `sudo` to allow an ordinary user to execute only _specific_ commands as a superuser
- Authenticating using `sudo` does not require the superuser's password by the user's own password
- It doesn't start a new shell or load another user's environment

The syntax looks like this:

```bash
sudo <command>
```

## `su` command

The `su` command is used to start a shell as another user. The command syntax looks like this:

```bash
su [-[l]] [user]
```

If the `-l` option is included, the resulting shell session is a login shell for the specified user which includes their environment and working directory. If the user is not specified, the superuser is assumed.

The `-l` may be abbreviated as `-`. To start a shell for the superuser, we would do this:

```bash
su -
```
