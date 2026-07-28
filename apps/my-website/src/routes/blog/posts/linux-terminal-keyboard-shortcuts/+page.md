---
title: "CHEATSHEET: Terminal Keyboard Shortcuts"
description: "A cheatsheet and reference for some of my most used bash keyboard shortcuts for things like copy pasting or cursor movement"
author: "Saikat Das"
tags:
  - reference
  - cheatsheet
  - linux
  - keyboard
  - shortcuts
  - bash
  - command-line
  - shell
  - terminal
date: "2020-03-26"
icon: "bash.png"
---

As a web developer, I spend a significant amount of time typing terminal commands when I'm not writing source code in my favourite editor. And keyboard shortcuts in the terminal differ significantly from keyboard shortcuts in my favourite code editor (😍 VS Code 😍).

I've listed here the keyboard shortcuts I find useful:

## Cursor Movement

- `Ctrl + a` - move cursor to the **beginning** of the **line**. I use the `Home` key for this
- `Ctrl + e` - move cursor to the **end** of the **line**. I use the `End` key for this
- `Ctrl + f` - move cursor **forward one character**. I use the `➡` right arrow key for this
- `Ctrl + b` - move cursor **backward one character**. I use the `⬅` left arrow key for this
- `Alt + f` - move cursor **forward one word**. I use `Ctrl + ➡` for this
- `Alt + b` - move cursor **backward one word**. I use `Ctrl + ⬅` for this
- `Ctrl + l` - clear the screen. `clear` does the same thing

## Cutting and pasting text

- `Ctrl + k` - cut text from the **cursor location** to the **end of the line**
- `Ctrl + u` - cut text from the **cursor location** to the **beginning of the line**
- `Alt + d` - cut text from the **cursor location** to the **end of the current word**
- `Alt + Backspace` - cut text from the **cursor location** to the **beginning of the current word**
- `Ctrl + y` - paste text into the current cursor location

## Process Management

- `Ctrl + c` - to send a *interrupt* signal to the process
- `Ctrl + d` - exit the program indiscriminately

## Other

- `!!` - entering this will rerun the last command. Can be used in conjunction with `sudo`
- `Ctrl + r` - reverse search. Will go through the last run commands in reverse order. Uses the bash history