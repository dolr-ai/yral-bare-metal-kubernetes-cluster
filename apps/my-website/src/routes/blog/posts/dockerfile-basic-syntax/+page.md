---
title: "CHEATSHEET: Dockerfile - Basic Syntax and Concepts"
description: "A cheatsheet for the basic commands used in a Docker configuration Dockerfile. Mostly useful when setting up own deployment images"
author: "Saikat Das"
tags:
  - docker
  - dockerfile
  - images
  - containers
  - kubernetes
  - virtualization
date: "2020-05-15"
icon: "docker.png"
---

## Instructions

- `FROM` - initializes a new build stage and sets the base image for subsequent instructions
- `RUN` - will execute any commands in a new layer on top of the current image and commit the results which will be used for the next step in the `Dockerfile`
- `CMD` - used to provide defaults for an executing container such as an executable and its arguments
- `COPY <src> <dest>` - copies new files or directories from `<src>` and adds them to the filesystem of the container at the path `<dest>`
- `WORKDIR` - sets the working directory to the specified directory for following instructions in the `Dockerfile`
