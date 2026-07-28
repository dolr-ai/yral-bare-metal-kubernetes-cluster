<script lang="ts">
  import { page } from "$app/stores";

  export let siteTitle: string;
  export let siteDescription: string;
</script>

<svelte:head>
  <title>{siteTitle}</title>
  <meta name="description" content={siteDescription} />

  <!-- Twitter Card data -->
  <meta name="twitter:card" content="summary" />
  <meta name="twitter:site" content="@saikatdas0790" />
  <meta name="twitter:title" content={siteTitle} />
  <meta name="twitter:description" content={siteDescription} />
  <meta
    name="twitter:image"
    content={`https://${$page.url.host}/assets/logos/logo-square-192.png`} />

  <!-- Open Graph data -->
  <meta property="og:url" content={`https://${$page.url.host}${$page.url.pathname}`} />
  <meta property="og:type" content="website" />
  <meta property="og:title" content={siteTitle} />
  <meta property="og:description" content={siteDescription} />
  <meta
    property="og:image"
    content={`https://${$page.url.host}/assets/logos/logo-square-192.png`} />
</svelte:head>
