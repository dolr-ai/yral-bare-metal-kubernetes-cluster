---
title: "CHEATSHEET: Docker Compose - Basic Syntax and Concepts"
description: "A cheatsheet for the Docker Compose config file and CLI for commonly used configuration and commands"
author: "Saikat Das"
tags:
  - docker
  - dockerfile
  - compose
  - images
  - containers
  - kubernetes
  - virtualization
date: "2020-05-17"
icon: "docker.png"
---

## Configuarion

- `version` - specify which version of docker compose file format to be used
- `services` - lists the different services to be started for the app. Each _service_ definition contains configuration that is applied to each container started for that service
  - `image` - specifies the image to start the container from
  - `build` - configuration options that are applied at build time
  - `ports` - expose ports from the container to the host machine
  - `restart` - sets the restart policy. Possible values are:
    - `"no"` - no restarts of container. This is the default. The double-quotes are required
    - `always` - restarts if container stops for **ANY** reason
    - `on-failure` - restarts if container stopped with an _error_ code
    - `unless-stopped` - restarts unless the container was manually stopped

## CLI Commands

- `docker-compose up` - builds, (re)creates, starts, and attaches to containers for a service. Accepted flags are:
  - `--build` - build images before starting containers
  - `-d, --detach` - run containers in the background
- `docker-compose down` - stops containers and removes containers, networks, volumes, and images created by up
- `docker-compose ps` - lists the running containers initialized by `docker-compose` with the current `docker-compose.yml` file
