---
title: "React Hooks: useEffect"
description: "A gentle introduction to the React Hook useEffect with a practical example showing how to use it in code and a comparison with the legacy approach of using componentDidMount, componentDidUpdate and componentWillUnmount"
author: "Saikat Das"
tags:
  - react
  - reactjs
  - hooks
date: "2019-09-02"
icon: "react.png"
---

## What is an Effect?

An effect, or "side effect" as popularly known, is anything that happens inside a function which doesn't relate directly to the function being executed. The side effect reaches out of the scope of the function and performs operations that are not limited to the executing function. For example,

- fetching data from a REST endpoint (a network request)
- making changes to the DOM
- setting up subscriptions on and unsubscribing from updates from particular values

are all examples of side effects.

## Side effects in React

The way React works, side effects are performed after the render function has finished executing.

This would ideally be inside one of the three lifecycle methods, `componentDidMount`, `componentDidUpdate` and `componentWillUnmount`. They execute like following:

1. On initial mount, the operations inside `componentDidMount` are executed. This is ideally where the initial operations like fetching initial page load data is done
2. On subsequent renders, the operations inside `componentDidUpdate` are executed. This is where any changes in data are checked and updated
3. Finally, when the component is removed (_unmounted_), any cleanup that we need to perform like unsubscriptions are performed in the `componentWillUnmount` method.

## `useEffect` to the rescue ✨

With the introduction of useEffect the same can be accomplished with a very intuitive API that does all of the three things listed above. We'll understand by extending the same example we've been using till now,

```javascript
import React, { useState, useEffect } from "react";

function Example() {
  const [count, setCount] = useState(0);

  useEffect(() => {
    document.title = `You clicked ${count} times`;
    return () => {
      document.title = "";
    };
  });

  return (
    <div>
      <p>You clicked {count} times</p>
      <button onClick={() => setCount(count + 1)}>Click me</button>
    </div>
  );
}
```

Here, the effect is setting the page title to the number of times the button has been clicked. `useEffect` runs on every render including the initial load. So, it is performing responsibilities of both the `componentDidMount` and `componentDidUpdate` lifecycle methods. Finally, the function returned from useEffect is called when the component is _unmounted_, hence, `componentWillUnmount` is also covered by useEffect

`useEffect` is a really neat little utility that every React developer should add to their React tool-belt as it makes handling component lifecycle a breeze. Think we can do better. Let me know in the comments.
