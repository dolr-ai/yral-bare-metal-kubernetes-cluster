---
title: "React: Code Splitting and Lazy Loading"
description: "Using React Router with Create React App to setup lazy loading and code splitting by using dynamic ES imports. This is demonstrated with an example"
author: "Saikat Das"
tags:
  - react
  - reactjs
  - codesplitting
  - lazyloading
  - router
  - reachrouter
date: "2019-09-05"
icon: "react.png"
---

## Why do we need to split code and lazy load it?

As we saw in our earlier post, in a single page React application, all the statically imported components are fetched on page load even though the user might only be visiting a single route that displays a particular component.

This is due to the fact that React projects typically use a bundler like [Webpack](https://webpack.js.org/) or [Parcel](https://parceljs.org/) to minify and bundle all their code. This leads to a single minified file getting created for the JavaScript code written which loads all the components that we're using.

## Code Splitting
Splitting up our code based on the sections a user visits and needs to load in a single go when browsing our website is known as **Code Splitting**. The easiest way to decide which components to code split is by routes. This is because users are already conditioned by traditional websites to expect some delay when navigating between links on a website.

## Lazy Loading
When we've decided which sections of our app to split, we can use the lazy loading API provided by React to lazy load them only when the user navigates to it.

## Example
We'll demonstrate this using the earlier router example that we've already seen:
```javascript
// Home.js
const Home = () => (
  <div>
    <h2>Welcome</h2>
  </div>
);
export default Home;

// Dashboard.js
const Dashboard = () => (
  <div>
    <h2>Dashboard</h2>
  </div>
);
export default Dashboard;
```
```javascript
// App.js
import React, { lazy } from "react";
import { render } from "react-dom";
import { Router, Link } from "@reach/router";

const Home = lazy(() => import('./Home'));
const Dashboard = lazy(() => import('./Dashboard'));

const App = () => (
  <div>
    <h1>Tutorial!</h1>
    <nav>
      <Link to="/">Home</Link>{" "}
      <Link to="dashboard">Dashboard</Link>
    </nav>
    
    <Router>
      <Home path="/" />
      <Dashboard path="/dashboard" />
    </Router>
  </div>
);

render(<App />, document.getElementById("root"));
```

Here, we are using the new *dynamic import* syntax with the React *lazy loading API* to tell *Webpack* to code split these modules and they will be lazy loaded on the user's browser when they navigate to the specific path.
Note that the modules to be dynamically imported have to be **default exports**.

That's it for code splitting and lazy loading. Do you think this is a step-forward in the way we structure our code or is this too complicated to bother with in our single page applications?
