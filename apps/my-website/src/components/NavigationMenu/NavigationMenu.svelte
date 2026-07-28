<script lang="ts">
import { WIDTH_TO_AUTO_OPEN_NAVIGATION_MENU_AT } from "$utils/constants";
  import { onMount } from "svelte";
  import HamburgerButton from "./HamburgerButton.svelte";
  import Navbar from "./Navbar.svelte";

  let menuOpen: boolean = false;
  onMount(() => {
    if (window.innerWidth >= WIDTH_TO_AUTO_OPEN_NAVIGATION_MENU_AT) {
      menuOpen = true;
    }
  });
</script>

<HamburgerButton bind:menuOpen />
{#if menuOpen}
  <Navbar bind:menuOpen />
{/if}
