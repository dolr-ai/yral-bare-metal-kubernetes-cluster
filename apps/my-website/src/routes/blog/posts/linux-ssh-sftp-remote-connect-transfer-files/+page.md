---
title: "SSH and SFTP: Using secure shell to connect to remote servers and SFTP to transfer files"
description: "A summary of using secure shell to connect to remote targets and run commands and SFTP to transfer files between the remote server and local computer"
author: "Saikat Das"
tags:
  - summary
  - linux
  - ssh
  - sftp
  - remote
  - command-line
  - shell
  - terminal
date: "2021-07-20"
icon: "bash.png"
---

We all invariably run into SSH when setting up our local workspaces to clone repositories from Github or another git provider. Git piggybacks on the SSH protocol to securely help us push code to remote repos. However, we can also use SSH to log into remote servers and execute commands based on permissions provided.

## Connecting to remote
The process for generating ssh keys is the same as documented here on [Github](https://docs.github.com/en/github/authenticating-to-github/connecting-to-github-with-ssh/generating-a-new-ssh-key-and-adding-it-to-the-ssh-agent). Once the keys are generated, we need to add our public key to the list of authorized ssh keys on the remote server. Once that is done, we can use the following to ssh into it:

```bash
ssh <username>@<remote-address>
```

Once connected, we are logged into the remote computer as the user mentioned above and we can perform actions that are permitted for that specific user

## Transferring files

When we've SSHed into the remote target, we can run `sftp` to be dropped into *sftp* mode. Our prompt change indicates the change in mode. In this mode, we are active in two folders, one on our local and one on the remote. Transfer operations occur relative to these 2 locations. The two main commands that we use here are:
- `put` - transfer files from local to remote
- `get` - transfer files from remote to local

Arguments to the above would be file paths from local and remote directories respectively. 

Examples would be:

```bash
put file1.txt #transfer file1.txt from local to remote PWD
get file1.txt #download file1.txt from remote to local PWD
```

These support all the **glob** patterns that we are already familiar with. This mode supports a **l** prefix before common commands such as `pwd` or `ls`. So, running `lpwd` would show the **local** *present working directory* and running plain `pwd` would show us the **remote** *present working directory*