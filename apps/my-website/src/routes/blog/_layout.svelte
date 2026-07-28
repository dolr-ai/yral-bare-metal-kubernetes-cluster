<script>
  import { page } from "$app/stores";

  export let title;
  export let description;
  export let author;
  export let tags;
  export let date;
  export let icon;
</script>

<svelte:head>
  <title>{title}</title>
  <meta name="description" content={description} />
  <meta name="keywords" content={tags.join(", ")} />
  <meta name="author" content={author} />

  <!-- Twitter Card data -->
  <meta name="twitter:card" content="summary" />
  <meta name="twitter:site" content="@saikatdas0790" />
  <meta name="twitter:title" content={title} />
  <meta name="twitter:description" content={description} />
  <meta
    name="twitter:image"
    content={`https://${$page.url.host}/assets/icons/post-topics/${icon}`} />

  <!-- Open Graph data -->
  <meta property="og:url" content={`https://${$page.url.host}${$page.url.pathname}`} />
  <meta property="og:type" content="article" />
  <meta property="og:title" content={title} />
  <meta property="og:description" content={description} />
  <meta
    property="og:image"
    content={`https://${$page.url.host}/assets/icons/post-topics/${icon}`} />
</svelte:head>

<article class="prose prose-emerald max-w-screen-md mx-auto my-8 px-4">
  <h1 class="!mb-2">{title}</h1>
  <span class="text-gray-400">
    published on <span class="text-emerald-600 font-medium">
      {new Date(date).toDateString()}
    </span>
  </span>

  <slot />
</article>
