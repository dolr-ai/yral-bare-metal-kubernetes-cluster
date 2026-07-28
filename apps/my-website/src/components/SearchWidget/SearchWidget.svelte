<script lang="ts">
  import { Index } from "lunr";
  import type { BlogPostCardDetails } from "$types";

  export let searchText = "";
  export let filteredPosts: BlogPostCardDetails[];
  export let blogPosts: BlogPostCardDetails[];
  let searchIndex: Index;
  let searchInput: HTMLInputElement;
  let searchResults: Index.Result[];

  $: isIndexLoaded = Boolean(searchIndex);
  let shouldShowWaitingIndicator = false;

  $: {
    if (isIndexLoaded && searchText) {
      searchResults = searchIndex.search(searchText);
    }
  }

  $: {
    if (isIndexLoaded && searchText && searchResults && searchResults.length) {
      filteredPosts = searchResults
        .map((result) => blogPosts.find((blogPost) => blogPost.slug === result.ref))
        .filter((post): post is BlogPostCardDetails => post !== undefined);
    } else {
      filteredPosts = blogPosts;
    }
  }
</script>

<style>
  input {
    transition: border-color 0.3s ease-in-out;
  }
</style>

<input
  type="text"
  placeholder={shouldShowWaitingIndicator
    ? "Fetching search index, sit tight"
    : "Keep typing to see search results..."}
  bind:value={searchText}
  disabled={shouldShowWaitingIndicator}
  bind:this={searchInput}
  class="block w-11/12 mx-auto my-6 border-b-2 border-gray-300 text-xl focus:outline-none focus:border-emerald-400 active:border-emerald-400"
  on:focus={async () => {
    if (!isIndexLoaded) {
      shouldShowWaitingIndicator = true;

      searchIndex = Index.load(
        await (
          await fetch(`/blog/getSearchIndex.json`, {
            method: "GET",
          })
        ).json(),
      );

      shouldShowWaitingIndicator = false;
      setTimeout(() => {
        searchInput.focus();
      }, 50);
    }
  }} />
