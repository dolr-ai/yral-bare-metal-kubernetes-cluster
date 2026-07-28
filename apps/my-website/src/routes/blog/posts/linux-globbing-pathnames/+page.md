---
title: "Globs: Using wildcards to match files and directories in Linux"
description: "A cheatsheet and reference for glob expansions in bash to expand wildcards and match file and folder pathnames. The different wildcards are illustrated with examples"
author: "Saikat Das"
tags:
  - linux
  - glob
  - wildcard
  - directory
  - file
  - bash
  - command-line
  - shell
  - terminal
date: "2020-03-17"
icon: "bash.png"
---

**Globbing** refers to using _wildcards_ and a system of identifiers to expand _search strings_ into the list of pathnames matching the search string pattern. This can give us superpowers when working with multiple files.

<script>
  import SuperpowerImage from "./superpower.webp";
</script>

<img src={SuperpowerImage} alt="Super power" class="mx-auto">


This is useful for us whenever we are working with multiple files and need to move, copy, delete them. We also use these patterns in other applications when specifying which files to **include** or which files to **ignore**. For example, we use these patterns in a `.gitignore` file to tell _Git_ which files to ignore

## Wildcards

- `*` - matches any character
- `?` - matches any single character
- `[...]` - matches any character in the square brackets
- `[!...]` - matches any character not in the square brackets
- `[[:class:]]` - matches any character that is a member of the specified _class_

**Note** - We can use `-` inside the square brackets to denote ranges like we do for regular expressions

Some common classes are listed below

### Common Classes

- `[:alnum:]` - matches any alphanumeric character
- `[:alpha:]` - matches any alphabetic character
- `[:digit:]` - matches any numeral
- `[:lower:]` - matches any lowercase letter
- `[:upper:]` - matches any uppercase letter

## Examples

- `*` - all files
- `**` - all files including ones inside subfolders
- `dir1/g*` - any file beginning with a _g_ inside _dir1_
- `dir2/b*.txt` - any file inside _dir2_ beginning with b followed by any characters and ending with _.txt_
- `Data???` - any file beginning with _Data_ followed by exactly 3 characters
- `dir1/[abc]*` - any file inside _dir1_ beginning with _a_, _b_ or _c_
- `BACKUP.[0-9][0-9][0-9]` - any file beginning with _BACKUP._ followed by exactly three numerals
- `[[:upper:]]*` - any file beginning with an uppercase letter
- `[![:digit:]]*` - any file not beginning with a numeral
- `*[[:lower:]123]` - any file ending with a lowercase letter or the numerals _1_, _2_ or _3_
