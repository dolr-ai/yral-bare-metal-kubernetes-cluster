---
title: "CHEATSHEET: Docker CLI - Basic Commands and Concepts"
description: "A cheatsheet for the Docker CLI to create and manage containers and images"
author: "Saikat Das"
tags:
  - docker
  - cli
  - containers
  - kubernetes
  - virtualization
date: "2020-05-15"
icon: "docker.png"
---

Docker is a gamechanger when it comes to scalable application development and deployment. However, as an individual developer, there are lots of benefits that you can get out of using it in your personal workflows. For example, installing any database doesn't require you to spend hours 😫 setting up its dependencies and resolving its quirks. You can just use Docker to get ready to use images for popular software packages. This is a list of commonly used Docker CLI commands.

## Creating containers

- `docker run -p <host_port>:<container_port> -v <host_path>:<container_path>` - create a container from an image and start the container
  - we use the `-p` flag to map a port on the host machine to a port on the docker container
  - we use the `-v` flag to **bind mount** a path on the host machine to a path on the container
- `docker create` - create a container from an image without running it
- `docker start` - start an already existing container

## Managing containers

- `docker ps` - get a list of currently running containers
- `docker ps -a` - get a list of all containers (including exited)
- `docker logs <container_id>` - gets the logs of the running container from their `stdout`
- `docker exec -it <container_id> <command>` - run a command interactively inside a running container and get `stdout` & `stderr` on the current terminal

## Removing containers

- `docker system prune` - removed all unused resources including stopped containers or networks, build caches not in use

## Creating images

- `docker build . -f <dockerfile_location> -t <dockerHubID/imageName:versionNumber>` - build an image using the current directory as the **context** and using the `Dockerfile` present at the root of the context
  - we use the `-f` flag to specify the `Dockerfile` location in case it's not in the root directory of the context
  - we use the `-t` flag to tag the generated image. The convention for tagging images is as above
- `docker commit` - create a new image from a container's changes
