<script lang="ts">
  import type { BlogPostCardDetails } from "$types";
  import SeoMetaHeader from "$components/SEO/SEOMetaHeader.svelte";
  import BlogPostCard from "$components/BlogPostCard/BlogPostCard.svelte";
  import SearchWidget from "$components/SearchWidget/SearchWidget.svelte";
  import { flip } from "svelte/animate";

  export let data: { blogPosts: BlogPostCardDetails[] };
  $: blogPosts = data.blogPosts;
  let filteredPosts = data.blogPosts;
</script>

<style>
  main > h1 {
    text-decoration-color: #6ee7b7;
    text-decoration-line: underline overline;
    text-underline-offset: 0.75rem;
    text-decoration-style: wavy;
  }
</style>

<SeoMetaHeader
  siteTitle="Saikat's Blog"
  siteDescription="This is my blog where I write about what I learn including things like SvelteJS, TailwindCSS, NodeJS, Linux, PostgreSQL among a few" />

<main class="max-w-screen-md mx-auto p-2">
  <SearchWidget bind:filteredPosts {blogPosts} />
  <h1 class="text-3xl text-center mt-16 mb-8">Blog Posts</h1>
  <ul>
    {#each filteredPosts as { created, icon, slug, tags, title, description } (slug)}
      <div animate:flip={{ duration: 500 }}>
        <BlogPostCard {created} {icon} {slug} {tags} {title} {description} />
      </div>
    {/each}
  </ul>
</main>
