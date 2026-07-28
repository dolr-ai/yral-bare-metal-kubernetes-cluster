<script>
  import { page } from "$app/stores";
  import { format } from "date-fns";

  export let title;
  export let description;
  export let technologiesUsed;
  export let author;
  export let startDate;
  export let endDate;
</script>

<svelte:head>
  <title>{title}</title>
  <meta name="description" content={description} />
  <meta name="author" content={author} />

  <!-- Twitter Card data -->
  <meta name="twitter:card" content="summary" />
  <meta name="twitter:site" content="@saikatdas0790" />
  <meta name="twitter:title" content={title} />
  <meta name="twitter:description" content={description} />
  <meta
    name="twitter:image"
    content={`https://${$page.url.host}/assets/logos/logo-square-192.png`} />

  <!-- Open Graph data -->
  <meta property="og:url" content={`https://${$page.url.host}${$page.url.pathname}`} />
  <meta property="og:type" content="article" />
  <meta property="og:title" content={title} />
  <meta property="og:description" content={description} />
  <meta
    property="og:image"
    content={`https://${$page.url.host}/assets/logos/logo-square-192.png`} />
</svelte:head>

<article class="prose prose-emerald max-w-screen-md mx-auto my-8 px-4">
  <h1 class="!mb-2">{title}</h1>
  <span class="text-gray-400">
    worked on between <span class="text-emerald-600 font-medium">
      {format(new Date(startDate), "MMM yyyy")}
    </span>
    to
    <span class="text-emerald-600 font-medium">
      {format(new Date(endDate), "MMM yyyy")}
    </span>
  </span>

  <slot />

  <h2>This Project Uses</h2>
  <ul>
    {#each technologiesUsed as technology (technology)}
      <li>{technology}</li>
    {/each}
  </ul>
</article>
