---
title: "The Knotty Tales"
description: "A platform for wedding vendors like photographers or makeup artists to list their services for visitors to discover. It includes a marketplace for vendors to purchase credits and redeem them for enquiry leads for reaching out with their services"
technologiesUsed:
  - ReactJS
  - GatsbyJS
  - Material UI
  - Firebase Firestore
  - Firebase Authentication
  - Firebase Cloud Functions
  - Firebase Storage
  - Firebase Cloud Messaging
  - Google Analytics
  - Facebook API
  - PayU Payment Gateway
  - Hubspot API
author: "Saikat Das"
coverPhoto: "the-knotty-tales/theknottytales-cover.jpg"
startDate: "2019-09-01"
endDate: "2020-03-30"
layout: "projects"
---

This is a Gatsby site built with React and Material UI and mostly Firebase for the backend. Most of the marketing and landing pages are statically rendered for the best SEO scores. The vendor dashboard and vendor discovery sections of the website are single page apps (SPAs). The vendor dashboard is behind a login and includes functionality like a payment portal to acquire credits, a leads marketplace where credits can be exchanged for leads, and mechanisms to update their profiles, calendar availability and upload portfolio photos.

Here's a video walkthrough going over the entire website:

<iframe src="https://www.youtube.com/embed/gNjVTMeOS7A" title="YouTube video player" frameborder="0" allow="accelerometer; autoplay; clipboard-write; encrypted-media; gyroscope; picture-in-picture" allowfullscreen loading="lazy" class="w-full" style="aspect-ratio: 16 / 9"></iframe>

## Highlights of the website include:
- All landing pages statically generated for fast loads from CDN. Uses **GatsbyJS** and **Firebase Hosting**
- Dynamic pages like dashboards and feeds are **single page apps (SPAs)** that fetch data on page load
- Infinite scroller that fetches cards lazily. Uses **intersection observer**
- Embedded gallery component for **image carousel**
- Availability calendar component
- **MDX** based static blog that renders static pages from markdown files
- Integration with Hubspot for **realtime chat widget** for inquiries and support
- Integration with **PayU payment gateway** for purchasing credits for vendors
- Leads Marketplace that displays partial (obfuscated) contact details with lead enquiry data without vendors having to log in
- Vendor dashboard profile section form to update profile details displayed on vendor's unique landing page
- Vendor portfolio section that lets them **upload, add descriptions to and delete images** for managing their public portfolio. Backend uses object storage and database to seamlessly **store uploaded images**, **process uploaded images** to **generate responsive images** for various devices and resolutions
- Vendor packages section that lets them upload different pricing packages that they have available for display on their public profile
- Vendor calendar availability section that provides a **calendar widget** that lets them set their calendar availability for display on their public profile
- Vendor payments section that lets vendors **purchase credits** with a payment gateway integration. The credits can be used to **unlock enquiry leads** for vendor partners to reach out with their offerings and services
- An entire marketplace of leads where lead pricing is dynamic. Lead pricing is determined using factors such as freshness of the lead and how many prior unlocks have been done by other vendors. All this is processed automatically on the backend for a seamless experience of the vendor partner.
- Gamified unlocking of leads to add a WOW factor to incentivize repeat engagement with the platform
- Leads unlocked history for a list of all unlocked leads
- Integration with **Firebase Cloud Messaging** to enable push notifications for vendor partners for them to receive notifications on availability of new leads
- The app is a **PWA** so that both mobile and desktop users can install the app and have it behave like a platform native app

## Link to website

Here's a [link](https://theknottytales.com/) to the website. However, this might be down in case the service has been retired