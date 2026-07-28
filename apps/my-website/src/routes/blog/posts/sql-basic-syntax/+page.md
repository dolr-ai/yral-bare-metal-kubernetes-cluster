---
title: "CHEATSHEET: SQL Syntax Basics"
description: "A cheatsheet of basic SQL syntax for common CRUD operations like querying, inserting and deleting data"
author: "Saikat Das"
tags:
  - sql
  - databases
  - relational
  - cheatsheet
date: "2020-06-09"
icon: "database.png"
---

This is a list of some basic SQL syntax that one would need to refer on a regular basis. This is relational database agnostic and any database supporting ANSI SQL syntax should support these. A more specific list listing Postgres commands to follow.

## Basic Query

```sql
SELECT column_name, ...
FROM table_name;
```

## Conditional Query

```sql
SELECT column_name, ...
FROM table_name
WHERE condition;
```

## Pattern Matching

```sql
SELECT column_name, ...
FROM table_name
WHERE column_name LIKE 'pattern';
```

## Sorted Search

```sql
SELECT column_name, ...
FROM table_name
WHERE condition
ORDER BY column_name;
```

## Aggregating and Grouping

```sql
SELECT column_name, ...
FROM table_name
WHERE condition
GROUP BY column_names_for_grouping
HAVING condition_for_grouped_rows
```

## Joining Tables

```sql
SELECT table_name1.column_name, ...
FROM table_name1,table_name2, ...
WHERE table_name1.column_name = table_name2.column_name
```

## Creating a Table

```sql
CREATE TABLE table_name(
column_name1 datatype,
column_name2 datatype,
...
);
```

## Creating a View

```sql
CREATE VIEW view_name
AS SELECT statement
```

## Deleting a Real Table

```sql
DROP TABLE table_name;
```

## Deleting a view

```sql
DROP VIEW view_name;
```

## Inserting a Row

```sql
INSERT INTO table_name(column_name1, ...)
VALUES (value1, ...)
```

## Updating a Row

```sql
UPDATE table_name
SET column_name = value1, ...
WHERE condition;
```

## Deleting a Row

```sql
DELETE FROM table_name
WHERE condition;
```
