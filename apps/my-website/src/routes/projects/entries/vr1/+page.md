---
title: "VR1 website"
description: "A slick and modern website for parents to check out details on the different creative coding courses provided by VR1. The website includes a way for parents to book calendar time slots to schedule demos and make online payments in their native currency"
technologiesUsed:
  - SvelteJS
  - Sapper
  - Google Cloud Run
  - Stripe Payments
  - Hubspot CRM API
  - Google Analytics
author: "Saikat Das"
coverPhoto: "vr1/vr1-cover.jpg"
startDate: "2020-08-03"
endDate: "2020-09-25"
layout: "projects"
---

This is a slick and modern website built with SvelteJS and Sapper. It is a server rendered (SSR) website that runs on a serverless product called Cloud Run from Google Cloud. The design was created in collaboration with a designer from Upwork on Figma. The entire website was crafted with attention to detail and focus on a vibrant and delightful UI. The booking page saves the submitter's data to Hubspot CRM and takes them to a calendar booking page with availability of the Demo Team. The pricing page is sensitive to the visitor's local currency. At launch we supported payments with **USD**, **CAD**, **INR** and **AED**. The backend detects the visitor's IP and accordingly updates the UI to show the visitor pricing in their native currency or **USD** in case their native currency integration is not available.

Here's a video walkthrough going over the entire website:

<iframe src="https://www.youtube.com/embed/Pn7eDFAa3y4" title="YouTube video player" frameborder="0" allow="accelerometer; autoplay; clipboard-write; encrypted-media; gyroscope; picture-in-picture" allowfullscreen loading="lazy" class="w-full" style="aspect-ratio: 16 / 9"></iframe>

## Highlights of the website include:
- All UI components are crafted and styled with basic HTML and CSS. No libraries used. This ensures a clutter free and snappy experience
- Embedded background video on the hero section of the landing homepage
- Booking form integrated with Hubspot CRM API to record form submitter's details even if they don't book a calendar slot. This is to help the demo team reach out to prospective customers personally and inquire if they're facing any difficulties setting up a demo.
- Interactive custom component for showcasing skill being taught
- Mobile responsive and highly detailed pricing grid that supports payment in multiple native currencies
- Stripe integration for multi currency support

## Link to website

Here's a [link](https://vr1-website-ialjghz3zq-uc.a.run.app/) to the website. However, this might be down in case the service has been retired