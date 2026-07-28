---
title: "Processes in Linux: Monitoring and Managing processes"
description: "Understanding processes in linux with utilities like ps, top and htop to monitor them. Using Linux utilities to manage processes by terminating or backgrounding them"
author: "Saikat Das"
tags:
  - process
  - monitor
  - manage
  - linux
  - bash
  - command-line
  - shell
  - terminal
date: "2020-04-02"
icon: "bash.png"
---

In Linux, **processes** are how all programs or commands run. When a system starts up, the kernel starts a program called `init`. `init` runs shell scripts which start all the system services. Some of them run only in the background and do not have a user interface. They are called _daemon_ processes. Programs launching other programes are called _parent_ and _child_ processes.

The _kernel_ assigns _process IDs_ in ascending order to processes with _init_ getting _PID 1_ and tracks memory assigned and resume execution readiness for these processes. Since everything in Linux is a file, processes also have owners, user IDs, etc.

## Viewing Processes

### Using the `ps` command

<script>
  import PSSnapshotImage from "./ps-snapshot.jpg";
  import TopSnapshotImage from "./top-snapshot.jpg";
  import HtopSnapshotImage from "./htop-snapshot.jpg";
</script>

<img src={PSSnapshotImage} alt="ps command output snapshot" class="mx-auto">

We can use the `ps aux` command to view a snapshot of currently running processes on the system. It shows us the processes with

- USER - owner of the process
- Process IDs assigned by the kernel
- % CPU - cpu usage
- %MEM - memory usage
- VSZ - virtual memory size
- RSS - amount of physical RAM being consumed
- CPU time consumed
- STAT - process states. Possible values are:
  - `R` - running or ready to run
  - `S` - sleeping, like waiting for an event such as a keystroke or a network packet
  - `D` - uniterruptible sleep. Like waiting for I/O such as a disk drive
  - `T` - stopped
  - `Z` - _zombie_ process. A child process that has terminated but has not been cleaned up by its parent
  - `<` - high-priority process
  - `N` - less-priority process

### Using the `top` command

<img src={TopSnapshotImage} alt="top command output snapshot" class="mx-auto" loading="lazy">

The `top` command shows a continuously updating display of the system processes listed in order of process activity.

The output has two parts &ndash; a system summary followed by a list of processes sorted by CPU activity.

The summary fields are explained below:

- Row 1 contains the
  - current time of the day
  - uptime for the machine
  - number of users logged in
  - average load which is the number of processes that are in a runnable state and sharing the CPU (values are average for the last **60 secs**, **5 mins** and **15 mins** respectively)
- Row 2 summarizes the number of processes and their various running states
- Row 3 describes the CPU utilization for
  - user processes - processes outside the kernel
  - system processes - kernel processes
  - nice processes
  - idle
  - waiting for I/O
- Row 4 shows physical RAM usage
- Row 5 shows virtual memory usage

### Using the `htop` command

<img src={HtopSnapshotImage} alt="htop command output snapshot" class="mx-auto" loading="lazy">

An alternative to the `top` command is the `htop` command which is more interactive and has more options for filtering, sorting and much more. It also has, in general, a more friendly and appealing user interface.

## Backgrounding Processes

To launch a program and immediately place it in the background, we follow the command with an ampersand (&) character.

```bash
<command> &
```

We can also launch a process, press `Ctrl + z` to pause it, and then resume it in the foreground using the `fg` command or in the background using the `bg` command.

We can use `jobs` to list the currently running background processes. The output would look something like:

```bash
[1] 31562
```

We can bring any background jobs to the foreground by using the `fg` command like this:

```bash
fg %<job_number>
```

Note that the job number here is different than the process ID. The job number in the above example is **1**, between the square brackets whereas the **31562** is the process ID

## Stopping Processes

We use the `kill` command to terminate processes by send a terminate signal to a process. We do it like so:

```bash
kill -signal <Process ID>
```

If no signal is specified, the `TERM` (terminate) signal is sent.

To send signals to multiple processes matching a specified program or username, we use the `killall` command like so:

```bash
killall [-u user] [-signal] name...
```

### Possible Signals

- `INT` - usually terminates a program. Keyboard shortcut is `Ctrl + c`
- `TERM` - program terminates. Default signal sent by `kill`
- `CONT` - continue a process after a `STOP` or `TSTP` signal. This is sent by the `bg` and `fg` commands
- `STOP` - pause a process without terminating
- `TSTP` - terminal stop, different from `STOP` in that the program may choose to ignore it. Keyboard shortcut is `Ctrl + z`
- `KILL` - kernel immediately terminates the process and the program is not given any opportunity to save or clean up. Should only be used as a last resort
- `HUP` - hang up. _Foreground_ programs running on the terminal will terminate. _Daemon_ programs reinitialize
