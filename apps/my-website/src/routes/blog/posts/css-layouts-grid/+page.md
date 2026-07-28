---
title: "CHEATSHEET: CSS Layouts - CSS Grid"
description: "A reference and a cheatsheet for CSS Grid. Goes over the main concepts and the syntax"
author: "Saikat Das"
tags:
  - reference
  - cheatsheet
  - css
  - layout
  - grid
date: "2020-02-15"
icon: "css.png"
---

## Terminology

- grid line
- grid track
- grid cell
- grid area
- grid container
- grid item

## Basic Syntax

### Parent container commands

#### Turn the parent into a grid container

```css
display: grid;
```

#### Setup rows and columns

```css
grid-template-rows: 100px 200px;
grid-template-columns: 200px auto;
```

#### Auto rows and columns

```css
grid-auto-rows: 100px;
grid-auto-columns: auto;
```

#### Flow direction

Controls how the auto-placement algorithm works

```css
grid-auto-flow: row;
grid-auto-flow: column dense;
```

#### Grid gutter and gap

```css
row-gap: 10px;
column-gap: 20px;

gap: 10px 20px;
```

### Child Item commands

#### Specifying track to occupy

```css
grid-row-start: 2;
grid-row-end: 4;
grid-column-start: 1;
grid-column-end: 2;

grid-row: 2/4;
grid-column: 1/2;

grid-row: 2 / span 2;
grid-column: 1 / span 1;

grid-area: 2/1/4/2;
```

#### Specifying display order of items

```css
order: -1;
order: 2;
```

## Display Areas

### Parent Container

```css
grid-template-areas:
  "header header"
  "body sidebar"
  "footer footer";
```

**Note:** To not take up a certain cell, specify that using `.` in that `grid-template-area`

### Child Items

```css
grid-area: header;
grid-area: body;
```

## Utility Functions

### [`repeat` function](https://developer.mozilla.org/en-US/docs/Web/CSS/repeat)

#### Fixed Repeat

```css
repeat(4, 250px);
```

#### Track repeat

```css
repeat(4, 1fr);
```

#### Auto repeat

```css
repeat(auto-fit, 250px);
```

### `auto-fit`

Largest possible whole repetitions without overflowing the grid container. Empty tracks are collapsed and distributed to items

### `auto-fill`

Same as auto fit except that empty tracks are displayed

### `minmax`

Defines a size range greater than or equal to min and less than or equal to max.

```css
minmax(200px, 1fr);
minmax(min-content, 200px);
```

## Units

### auto

- As a maximum, identical to `max-content`
- As a minimum, represents the largest minimum size of the grid items occupying the grid track

### [The `fr` unit](https://developer.mozilla.org/en-US/docs/Web/CSS/flex_value)

The `fr` unit represents a fraction of the leftover space in the grid container. Use `fr` with auto to make it expand with `auto` taking up as much space as it needs

### `max-content`

Represents the largest max-content contribution of the grid items occupying the grid track

### `min-content`

Represents the largest min-content contribution of the grid items occupying the grid track

## Alignment

### Directions

- `justify` along the **inline** axis
- `align` along the **block** axis

### Properties

- `content` - apply to container to affect tracks of the container
- `items` - apply to container to affect the items
- `self` - apply to items to affect items

### Values

- `auto`
- `center`
- `flex-start`
- `flex-end`
- `stretch`

#### Applies to `content` and `items`

- `baseline`
- `first baseline`
- `last baseline`

#### Applies to `content`

- `space-around`
- `space-between`
- `space-evenly`
