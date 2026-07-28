<script lang="ts">
  import { fly } from "svelte/transition";
  import HomeIcon from "$icons/flat-color-icons/home.svelte";
  import BlogIcon from "$icons/flat-color-icons/news.svelte";
  import ProjectsIcon from "$icons/flat-color-icons/deployment.svelte";
  import { WIDTH_TO_AUTO_OPEN_NAVIGATION_MENU_AT } from "$utils/constants";

  const navigationLinks = [
    { textToDisplay: "About Me", url: "/", icon: HomeIcon },
    {
      textToDisplay: "Blog",
      url: "/blog",
      icon: BlogIcon,
    },
    {
      textToDisplay: "Things I've built",
      url: "/projects",
      icon: ProjectsIcon,
    },
  ];

  export let menuOpen: boolean;
</script>

<aside
  class="fixed bottom-8 right-8 w-64 h-96 shadow rounded-lg bg-gray-50"
  transition:fly={{ duration: 500, x: 400, y: 400 }}>
  <nav>
    <ul>
      {#each navigationLinks as { textToDisplay, url, icon } (textToDisplay)}
        <a
          data-sveltekit-preload-data="hover"
          href={url}
          class="block py-2 px-6 text-lg text-emerald-800 hover:text-emerald-600 active:text-emerald-600 hover:bg-gradient-to-tr hover:from-emerald-100 hover:via-blue-100 hover:to-fuchsia-100 active:bg-gradient-to-tr active:from-emerald-100 active:via-blue-100 active:to-fuchsia-100"
          on:click={() => {
            if (window.innerWidth < WIDTH_TO_AUTO_OPEN_NAVIGATION_MENU_AT) {
              menuOpen = false;
            }
          }}>
          <li class="h-10 flex flex-row items-center">
            <svelte:component this={icon} className="h-5 mr-3" />
            <span>{textToDisplay}</span>
          </li>
        </a>
      {/each}
    </ul>
  </nav>
</aside>
