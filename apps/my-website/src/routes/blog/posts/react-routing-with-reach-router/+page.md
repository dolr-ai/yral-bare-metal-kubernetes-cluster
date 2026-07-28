---
title: "Routing in React"
description: "Client side routing in React with Reach Router. Goes over setting it up with an example"
author: "Saikat Das"
tags:
  - react
  - reactjs
  - hooks
date: "2019-09-04"
icon: "react.png"
---

## Primer on client-side Routing

Traditionally, **routing** on the web has relied on *URLs* to navigate between pages. Put simply, when a URL is put into the browser, the browser renders a page based on the results returned by the server. On subsequent navigation to other pages on the site, the browser performs a full-page load on the page navigated to including parts of the page that didn't change such as the *header* and/or the *footer*

With the advent of single page applications and client-side routing, we use JavaScript now to only load in sections of the page that need to be changed. This is beneficial as it both reduces load on the server and is light on the network usage on the client device.

<script>
  import WooHooImage from "./woohoo.gif";
</script>

<img src={WooHooImage} alt="Woo hoo" class="mx-auto">


React, being a modern, modular library, has its pick of client-side routers. We'll look at [Reach Router](https://reach.tech/router) in an example:
```javascript
// App.js
import React from "react";
import { render } from "react-dom";
import { Router, Link } from "@reach/router";
import { Home, Dashboard } from "./components.js";

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

```javascript
// components.js
const Home = () => (
  <div>
    <h2>Welcome</h2>
  </div>
);

const Dashboard = () => (
  <div>
    <h2>Dashboard</h2>
  </div>
);
```

This code renders two navigation links and the `Router` component on load. The `Router` component inspects the URL to check if it matches with any of the `path` values inside it. In this case, the "/" (root) path matches and the router renders the corresponding Home component
If we click the `Dashboard` link, that will match the "/dashboard" path and the corresponding Dashboard component will replace the currently displayed Home component. The navigation section doesn't need to be reloaded in this case

Note that, **both** the `Home` and the `Dashboard` component is loaded by the browser on initial page load due to the static nature of our imports in this case.

This is a fantastic use case for **code-splitting** and **lazy loading** our components which we'll cover in detail in another article.

Think client-side routing is the bomb or are you sceptical of its benefits and wary of the SEO hits that arise from it?
