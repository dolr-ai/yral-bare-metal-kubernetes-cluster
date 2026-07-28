---
title: "I/O Redirection in Linux"
description: "Using I/O redirection in Linux to take output from running any command or executing a program and either saving it to a file or passing it along to another program for further processing"
author: "Saikat Das"
tags:
  - linux
  - redirection
  - pipeline
  - bash
  - command-line
  - shell
  - terminal
date: "2020-03-22"
icon: "bash.png"
---

Linux I/O Redirection is a very powerful ability as you can virtually take output from running any command or executing a program and either save it to a file or pass it along to another program for further processing.

But it is up to us to send the right data through the right pipe.

<script>
  import PandasPipeImage from "./pandas-pipe.gif";
</script>

<img src={PandasPipeImage} alt="Pandas playing with pipes" class="mx-auto">


Since "everything in Linux is a file", there are special files called **standard output (stdout)**, **standard input (stdin)** and **standard error (stderr)**. **stdout** and **stderr** default to the current display device and **stdin** defaults to the keyboard. _I/O Redirection_ lets us redirect where these input and output are sent.

## Mental Model

The way to think about these tools is that we can feed the contents of a file to a program via standard input, dump the output of a program to a file via standard output or pass the output of a program to another program using pipes

## Redirection `>` operator

Using the `>` operator we can send the output of a program to a file. We can use the `>>` operator to append the output to the end of a file instead of overwriting the current contents which the `>` operator does.

### Redirecting Standard Error

A program can produce output on any of several numbered file streams. While we have referred to the first three of these file streams as **standard input**, **output** and **error**, the shell references them internally as file descriptors 0, 1 and 2 respectively. The shell provides a notation for redirecting files using the file descriptor number.

We can redirect standard error with this notation:

```bash
ls -l /bin/usr 2> ls-error.txt
```

### Redirecting Standard Output and Standard Error to One File

We can redirect both standard output and error to another file with this notation:

```bash
ls -l /bin/usr &> ls-output.txt
```

## Redirection `<` operator

This takes a file and feeds that into a program as standard input. For example:

```bash
grep "some-file" < ls-output.txt
```

## Pipelines

Using the pipe (`|`) operator, the standard output of one command can be piped into the standard input of another. For example, we could have the following:

```bash
ls /bin /usr/bin | sort | uniq | less
```
