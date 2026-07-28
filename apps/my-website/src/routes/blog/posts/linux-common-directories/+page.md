---
title: "Common Linux Directories and their purpose"
description: "Exploring and understanding the different folders and their common purposes in the root directory of a Linux filesystem"
author: "Saikat Das"
tags:
  - linux
  - directory
  - file
  - bash
  - command-line
  - shell
  - terminal
date: "2020-03-16"
icon: "bash.png"
---

Have you looked at the output of `ls` on your Linux root (`/`) directory and wondered what all of it was?


<script>
  import ConfusedImage from "./confused.gif";
</script>

<img src={ConfusedImage} alt="Confused" class="mx-auto">


Fret no more. In this article, we'll go over some of the common folders in Linux and what their intended purpose is.

Before we dig into the list, let's understand a few things about the Linux filesystem that differs from other filesystems like that on a Windows.

In Linux,

> Everything is a file

What this means is that all resources recognized by the operating system is represented as a node in its file tree structure. This also includes things like the currently running processes on the system or the hardware devices connected to the system.

The tree starts with one node known as the _root_ (`/`) node and everything else is part of the parent-child relationship under it.

## Common Folders

- `/` - root directory where everything begins
- `/boot` - contains the Linux kernel, initial RAM disk image and the bootloader
- `/bin` - contains binaries required for the system to boot and run
- `/dev` - short for devices. contains _device nodes_
- `/etc` - short for etcetera. contains all the system-wide configuration files.
- `/home` - contains a folder for each user of the system. Users are limited to writing files in their home directories
- `/lib` contains shared library files used by the core system programs
- `/lost+found` - used in case of a partial recovery from a file system corruption event
- `/media` - used for removable media such as USB drives that are mounted automatically at insertion
- `/mnt` - contains mount points for removable devices that have been mounted manually. Like additional hard disks connected to the system
- `/opt` - short for optional. Used to install commercial software products installed to the system
- `/proc` - short for processes. Virtual file system maintained by the kernel for processes running on the system
- `/root` - home directory for the **root** account
- `/sbin` - contains system binaries that perform vital tasks that are reserved for the superuser
- `/tmp` - contains temporary, transient files created by various programs
- `/usr` - short for user system resources. Contains all the programs and support files used by regular users
- `/usr/bin` - contains the executable programs installed by the particular Linux _distribution_
- `/usr/lib` - contains the shared libraries for the programs in `/usr/bin`
- `/usr/local` - used for programs that are not included with the distribution but are intended for system-wide use. This would include programs compile from source
- `/usr/share` - contains all the shared data used by programs from `/usr/bin` such as icons, backgrounds, etc.
- `/var` - used to store data that will change. Includes databases, user mail, etc.
- `/var/log` - contains _log_ files, records of various system activity
