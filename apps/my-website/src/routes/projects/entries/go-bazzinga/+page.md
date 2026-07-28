---
title: "Go Bazzinga App"
description: "GoBazzinga is a social media and content speculation platform built on blockchain, where users are rewarded using blockchain-based tokens for participating in challenges, interacting with content, speculating on the performance of the participating creators or simply spending time on the platform."
technologiesUsed:
  - SvelteJS
  - SvelteKit
  - TypeScript
  - TailwindCSS
  - PostgreSQL
  - Prisma
  - Docker
  - Google Cloud Run
  - Google Cloud SQL
  - Google Cloud Tasks
  - Firebase Authentication
  - Cloudflare Stream
  - Cloudflare Reverse Proxy
author: "Saikat Das"
coverPhoto: "go-bazzinga/go-bazzinga-cover.jpg"
startDate: "2021-04-25"
endDate: "2021-07-19"
layout: "projects"
---

<script>
  import CodeSnapshotImage from "./code-snapshot.jpg";
</script>


This is primarily a short video platform where any user can post a challenge for other users to participate. Participations can then be upvoted by users to decide winners with the highest amount of upvotes.

Here is what the current code statistics look like:

<img src={CodeSnapshotImage} alt="Code Statistics" class="mx-auto">


## Highlights of the website include:
- built using the cutting edge **SvelteKit** framework
- The frontend uses **Tailwind** for styling
- Frontend code is written in SvelteJS with **TypeScript** support
- It is a server rendered app (SSR) that is hosted on infinitely horizontally scalable stateless **Docker** containers running on Google **Cloud Run**
- The API layer uses **SvelteKit Endpoints** to expose REST endpoints. Since the same project hosts both frontend and backend, they are on the same domain and hence no CORS worries
- These REST endpoints talk to a **PostgreSQL** database hosted on Google **Cloud SQL** via **Prisma** which acts as our data access layer.
- We also use Google **Cloud Tasks** for asynchronously processing delayed jobs
- We integrate with **Google Login** and **Facebook Login** using **Firebase Authentication**
- All uploaded videos are hosted and processed on **Cloudflare Stream**
- All requests to the app are routed through **Cloudflare**

## Link to website

Here's a [link](https://gobazzinga.io/) to the website.