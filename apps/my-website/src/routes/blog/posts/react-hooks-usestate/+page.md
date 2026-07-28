---
title: "React Hooks: useState"
description: "A gentle introduction to the React Hook useState with a practical example showing how to use it in code and a comparison with the legacy approach of using this.setState"
author: "Saikat Das"
tags:
  - react
  - reactjs
  - hooks
date: "2019-08-29"
icon: "react.png"
---

Before _hooks_ came around, we've mostly been using function components to write components **without** any state. This would mean a component with HTML markup and data coming in from a parent component through props. This is the fundamental one-way data flow in React where data flows from the top into children components.

Now, if any of these components needed to have its own data, specific to itself or its children, also known as "_state_" in React land, it would need to be a class components. So, while writing components in React, one would start off with a function component for [prescribed benefits](https://www.google.com/search?q=react+classes+vs+functions) but would ultimately need to convert to a class component in case they needed their component to be stateful. This would need copious amounts of refactoring and being mindful of the revered **_this_** :neutral_face:

We'll revisit the example from the React Docs referenced in my original article available [here](https://www.linkedin.com/pulse/react-hooks-overview-saikat-das/)

```javascript
import React, { useState } from "react";

function Example() {
  // Declare a new state variable, which we'll call "count"const [count, setCount] = useState(0);

  return (
    <div>
      <p>You clicked {count} times</p>
      <button onClick={() => setCount(count + 1)}>Click me</button>
    </div>
  );
}
```

Here, we are using the _useState_ hook to set some state for this component. The state here is a _count_ variable which holds the value of how many times the button has been clicked. The _setCount_ variable is a function, also returned by the _useState_ hook that lets us set the value for the _count_ variable.

And here, we have the class component that does the same thing

```javascript
import React from 'react';

class Example extends React.Component {
  constructor(props) {
    super(props);
    this.state = {
      count: 0
    };
  }

  render() {
    return (
      <div>
        <p>You clicked {this.state.count} times</p>
        <button onClick ​={() => this.setState({ count: this.state.count + 1 })}>
          Click me
        </button>
      </div>
    );
  }
}
```

😟 That's how it makes me feel when I look at the class component after having written the hooks function component.

The other thing to note over here, in class components, is that state is always an object and the values we store are saved as properties on that object.
Hook states on the other hand can be any JavaScript value, be it the native types (_number_, _text_) or data structures (_object_, _array_)

For example, all of these are valid `useState` calls

```javascript
function ExampleWithManyStates() {
  // Declare multiple state variables!
  const [age, setAge] = useState(42);
  const [fruit, setFruit] = useState("banana");
  const [todos, setTodos] = useState([{ text: "Learn Hooks" }]);
}
```

This new way of doing things just makes me a more productive developer. What do you think? Let me know in the comments
