---
title: "cURL and wget: Using cURL or wget to make requests to an endpoint"
description: "A summary of using cURL or wget to make different requests like GET or POST to an endpoint and inspect the response received"
author: "Saikat Das"
tags:
  - reference
  - linux
  - cURL
  - wget
  - REST
  - API
  - endpoints
  - shell
  - terminal
date: "2021-07-21"
icon: "bash.png"
---

As web developers, we often have to hit API endpoints manually to check the response before we build a frontend for it. Our first instinct might be to reach for a GUI tool like [Postman](https://www.postman.com/) or [Insomnia](https://insomnia.rest/). However, there's an easier way using cURL that is already available on most Linux based systems that we develop on.

## Using `cURL`

`cURL` works like a conventional Linux program and its output can be piped to other programs or redirected to files. So, it can be used to save something like `JSON` responses to a file to inspect later.

Example of `cURL` requests are:

```bash
# GET request to <url>
curl <url>

# POST request to <url>
curl --request POST <url>

# POST request with body. body is form-urlencoded
curl --request POST --data "some data" <url>

# POST request with header and JSON body
curl --header "Content-Type: application/json" \
  --request POST \
  --data '{"username":"xyz","password":"xyz"}' \
  https://example.com/login
```

### Common `cURL` flags to know

- `--request` - specify the request type such as `GET`, `POST`, `PUT`, `DELETE`, etc.
- `--data` - data being passed in body
- `--cookie` - send cookie from string/file
- `--location` - follow redirects

**PRO TIP:** To reverse engineer the `cURL` command for a particular request our browser makes, we can get it from the network tab of the Chrome browser console like below.

<script>
  import CurlProTip from "./curl-pro-tip.jpg";
</script>

<img src={CurlProTip} alt="Copying curl command from the browser network tab" class="mx-auto" loading="lazy"/>

## Use case for `wget`

I use `cURL` for almost all API request purposes. The only time I find myself using `wget` is when I need to recursively download a website or a specific directory on a domain. This is something `cURL` can't do. The syntax for this looks like:

```bash
wget --recursive <URL_to_download>
```