<script lang="ts">
  import BlogPostCard from "$components/BlogPostCard/BlogPostCard.svelte";
  import type { BlogPostCardDetails } from "$types";

  export let recentPosts: BlogPostCardDetails[];
</script>

<style>
  h1 {
    text-decoration-color: #6ee7b7;
    text-decoration-line: underline overline;
    text-underline-offset: 0.75rem;
    text-decoration-style: wavy;
  }
</style>

<h1 class="text-4xl text-center mb-8 mt-20">Recent Blog Posts</h1>
<ul>
  {#each recentPosts as { created, icon, slug, tags, title, description } (slug)}
    <BlogPostCard {created} {icon} {slug} {tags} {title} {description} />
  {/each}
</ul>
