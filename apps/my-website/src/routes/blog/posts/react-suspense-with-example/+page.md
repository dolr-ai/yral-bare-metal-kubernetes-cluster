---
title: "React: Suspense and Error Boundaries"
description: "Using React Suspense to render a conditional UI. React Suspense lets you render a particular component while waiting for another component to load. This is demonstrated with an example"
author: "Saikat Das"
tags:
  - react
  - reactjs
  - suspense
  - errorhandling
date: "2019-09-06"
icon: "react.png"
---

## React Suspense

Suspense in React is an API to render conditional UI. It lets you render a particular component while waiting for another component to load. This can be useful in many scenarios, say

- Your user is waiting for a page section to load. You can show an animated loader while it loads
- You are displaying a complicated visualization that requires some backend to return data after cruncing some numbers. You can show an overview of the original data while it loads

To understand the API for React Suspense, we'll look at a simple Suspense in an example

```javascript
const OtherComponent = React.lazy(() => import("./OtherComponent"));

function MyComponent() {
  return (
    <div>
      <Suspense fallback={<div>Loading...</div>}>
        <OtherComponent />
      </Suspense>
    </div>
  );
}
```

Here, we are wrapping a lazily loaded component with a `Suspense` and displaying a `fallback` till the time it's not loaded. Naturally, a `Suspense` only makes sense for lazily loaded components, as statically loaded components would already be available on page load.

That's it for React Suspense. What do you think of this newer succinct API for conditionally rendering components? Do you prefer the older way of using higher-order components?
