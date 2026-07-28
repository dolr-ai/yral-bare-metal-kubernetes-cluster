---
title: "CHEATSHEET: Regular Expressions"
description: "A reference and a cheatsheet for Regular expressions. Goes over the main concepts and the syntax"
author: "Saikat Das"
tags:
  - reference
  - cheatsheet
  - regular-expressions
  - regex
date: "2020-02-14"
icon: "regex.png"
---

## Basics

### Tooling

[Regex 101](https://regex101.com/)

### Syntax

```
/regex/mode;
```

### Literal Characters

All the characters ranging from a-z, A-Z and 0-9 are considered literal. In case we want to match things exactly, we use these

### Regex Modes

- /\<regex\>/g - global
- /\<regex\>/i - case insensitive
- /\<regex\>/s - single line
- /\<regex\>/m - multiline
- /\<regex\>/ - no mode

## Metacharacters

### All metacharacters

```
\ - backslash
^ - caret
| - pipe
. - period
$ - dollar
? - question
* - asterisk
+ - plus
() - parantheses
[] - square braces
{} - curly braces
```

### Escaping metacharacter

Use a backslash `\` to escape metacharacters

### Other Metacharacters

```
\d - matches any decimal digit (0-9)
\D - matches any character other than a decimal digit

\w - matches any word character and underscore (A-Z, a-z, 0-9, _)
\W - matches any non-word character

\s - matches any white-space character
\S - matches any non white-space character
```

### The `.` metacharacter

`.` or period matches all except new line. It is also called a wild card as it will match any character.

### Non Printable characters

Use these to match non printable characters

```
\n - line feed
\t - tab character
\r - carriage return
\f - form feed
\v - vertical tab
```

**Note** - _Windows uses `\r\n` to terminate lines and Unix uses `\n`_

## Character Set Or Character Class

A character set matches only one character out of this set

```
[character set]
```

Eg.

```
[bc]at will match bat and cat
```

### Range

We use a hyphen inside a character class to specify a range of characters

#### Syntax

```
[start-end]

[A-Z] will match any uppercase alphabet
[a-z] will match any lowercase alphabet
[0-9] will match any number
```

### Negating a character set

We use `^` also known as the caret to negate a character set

```
[^abc] - any character other than abc
[^123] - any character other than 1 2 3
[^a-z] - any character other than lowercase a to z
```

### Metacharacters inside a character set

To use metacharacters inside a character set, use the escape `\` character before it

## Repetition

### Quantifiers

With a quantifier, we can detect how many instances of an element - a character, character set or a group - are present. They are:

```
? - matches the previous element zero or one time
* - matches the previous element zero or more times
+ - matches the previous element one or more times
```

### Limiting Repetition

We can control repetition with `{}`

```
{n} - matches the previous element exactly n times
{min,} - matches the previous element at least n times
{min,max} - matches at least n times, no more than m times
```

### Greedy and Lazy Expressions

`? + *` are by default greedy. They match the previous element as many times as possible. We can also change their behaviour to match the previous element as few times as possible

To do so, we need to add `?` after the **quantifier**

We can also achieve the same effect by negating the closing character with the quantifier. Like

```
<[^>]+>

Here the caret is negating the closing arrow with the plus quantifier
```

## Start and End

### Anchors

Anchors specify a position in the string where a match must occur

```
^ - match must occur at the beginning
$ - match must occur at the end
```

### Word Boundaries

```
\\b - match must occur on a boundary between a \w (alphanumeric) and a \W (non-alphanumeric) character
\B - match must not occur on a \\b boundary
```

## Groups

We use groups to apply a quantifier to the entire group or to restrict alternation to part of the regex. We can also create a numbered group and use it later. Syntax:

```
(group)
```

### Alternation

Alternation is done with the `|` pipe symbol. It is similar to character class in that we use it to match a single regex out of two or more

```
/gr[ae]y/
/gr(a|e)y/

```

both of the above match _gray_ and _grey_

### Nesting in alternation

We can nest sub alternates in alternates in groups. Like:

```
/(alt1(sub1|sub2)|alt2(sub1|sub2))/
```

### Numbered and Non Capturing Groups

- Groups in regex get numbered and can be reference like so `\1`
- To create a non capturing group not considered in the count, we need to add `?:` to the start of the group after the `(`

## Lookaround

To match if a certain _element A_ comes before or after an _element B_

### Lookahead

#### Positive

```
elementB(?=elementA)
```

#### Negative

```
elementB(?!elementA)
```

### Lookbehind

#### Positive

```
(?<=elementA)elementB
```

#### Negative

```
(?<!elementA)elementB
```
