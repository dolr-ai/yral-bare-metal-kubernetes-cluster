---
title: "Expansions in Linux"
description: "The different kinds of bash expansion utilities in the bash shell and how to use them with examples"
author: "Saikat Das"
tags:
  - linux
  - expansion
  - bash
  - command-line
  - shell
  - terminal
date: "2020-03-25"
icon: "bash.png"
---

When we run a command on Linux, there are quite a few substitutions that are done by _bash_ before execution. The process that makes this happen is called **expansion**.

<script>
  import HulkExpandingImage from "./hulk-expanding.gif";
</script>

<img src={HulkExpandingImage} alt="Hulk expanding" class="mx-auto">


## Pathname Expansion

The mechanism by which wildcards work is called pathname expansion. For examples on how to use, check out this [post](/blog/posts/linux-globbing-pathnames/)

## Tilde Expansion

_Tilde_ (`~`) expansion, by default, points to the home directory of the current user.

## Arithmetic Expansion

Arithmetic expansion supports basic arithmetic such as addition (`+`), subtraction (`-`), multiplication (`*`), division (`/`), modulo (`%`) and exponentiation (`**`). The syntax of arithmetic expansion looks like this:

```bash
$((expression))

echo $(($((5**2)) * 3))
```

## Brace Expansion

This is mostly useful to make a list of files or directories. It works like this:

```bash
echo Front-{A,B,C}-Back
Front-A-Back Front-B-Back Front-C-Back

echo Number_{1..5}
Number_1 Number_2 Number_3 Number_4 Number_5
```

## Parameter Expansion

_Variables_ stored by the system are expanded by bash to get their values. For example, `USER` is a variable containing our username. We can expand it like this:

```bash
echo $USER
saikat
```

## Command Substitution

This allows us to use the output of a command as an expansion. For example, we could

```bash
ls - $(which cp)
-rwxr-xr-x 1 root root 146880 Feb 28  2019 /bin/cp
```

## Escaping expansion

In case we wish to escape bash expansion, we can put the string section that is likely to be expanded inside double (`"`) quotes. For example:

### Without quotes

```bash
echo $(cal)
March 2020 Su Mo Tu We Th Fr Sa 1 2 3 4 5 6 7 8 9 10 11 12 13 14 15 16 17 18 19 20 21 22 23 24 25 26 27 28 29 30 31
```

### With quotes

```bash
echo "$(cal)"
     March 2020
Su Mo Tu We Th Fr Sa
 1  2  3  4  5  6  7
 8  9 10 11 12 13 14
15 16 17 18 19 20 21
22 23 24 25 26 27 28
29 30 31
```
