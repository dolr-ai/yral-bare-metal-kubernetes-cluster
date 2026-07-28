<script lang="ts">
  import { page } from "$app/stores";
</script>

<main class="max-w-screen-md mx-auto p-2">
  <img
    src="/assets/images/routes/__error/pupper.jpg"
    alt="Sorry looking pupper"
    class="w-full" />
  <p class="text-xl">
    Hey there, time traveller! You seem to have visited this page in the past
    when it doesn't exist yet! ({$page.status})
  </p>
  <p>
    Why not visit again in the future when this is available? In the meantime,
    why not <a
      data-sveltekit-preload-data="hover"
      href="/blog"
      class="text-fuchsia-500 font-semibold">
      check out
    </a>
    some of our recent posts on full stack Javascript
  </p>
</main>
