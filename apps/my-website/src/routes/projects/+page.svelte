<script lang="ts">
  import type { ProjectEntryCardDetails } from "$types";
  import SeoMetaHeader from "$components/SEO/SEOMetaHeader.svelte";
  import ProjectEntryCard from "$components/ProjectEntryCard/ProjectEntryCard.svelte";

  export let data: { projectEntries: ProjectEntryCardDetails[] };
  $: filteredPosts = data.projectEntries;
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
  siteTitle="Things I've Built"
  siteDescription="This is my portfolio of sorts where I document and showcase things I've built, going over their highlights or salient points." />

<main class="max-w-screen-md mx-auto p-2">
  <h1 class="text-3xl text-center my-8">Things I've Built</h1>
  <ul>
    {#each filteredPosts as { title, description, technologiesUsed, startDate, endDate, coverPhoto, slug } (slug)}
      <ProjectEntryCard
        {title}
        {description}
        {technologiesUsed}
        {startDate}
        {endDate}
        {coverPhoto}
        {slug} />
    {/each}
  </ul>
</main>
