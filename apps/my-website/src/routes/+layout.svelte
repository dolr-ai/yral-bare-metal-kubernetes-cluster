<script>
  import NavigationMenu from "$components/NavigationMenu/NavigationMenu.svelte";
  import ServiceWorkerLoader from "./_index/components/ServiceWorkerLoader.svelte";
  import "../app.css";
</script>

<slot />
<NavigationMenu />
<ServiceWorkerLoader />
