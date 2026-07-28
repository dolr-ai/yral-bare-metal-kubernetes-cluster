<script lang="ts">
  export let created: string;
  export let icon: string;
  export let slug: string;
  export let tags: string[];
  export let title: string;
  export let description: string;
</script>

<style>
  @keyframes rotate {
    to {
      --angle: 360deg;
    }
  }

  a:hover {
    --angle: 0deg;
    border-image: linear-gradient(var(--angle), #34d399, #ec4899) 1;
    animation: 2s rotate linear infinite;
    border-radius: 0.5rem;
  }

  @property --angle {
    syntax: "<angle>";
    initial-value: 0deg;
    inherits: false;
  }

  @media (min-width: 768px) {
    a > li {
      grid-template-columns: auto 1fr;
      grid-template-rows: repeat(4, auto);
    }
  }
</style>

<a
  href={slug}
  data-sveltekit-preload-data="hover"
  class="border-4 rounded-lg border-transparent block my-2 p-2 md:py-6">
  <li class="grid grid-cols-1">
    <img
      src={`/assets/icons/post-topics/${icon}`}
      alt={`${icon.replace(".svg", "")} icon`}
      class="justify-self-center h-32 my-8 md:row-span-4 md:w-24 md:h-auto md:mr-8 md:ml-4 md:my-0 md:self-center"
      loading="lazy" />
    <h2 class="font-bold text-2xl">{title}</h2>
    <span class="text-gray-400">
      published on <span class="text-emerald-600 font-medium">
        {new Date(created).toDateString()}
      </span>
    </span>
    <p class="my-2">{description}</p>
    <span class="text-fuchsia-600">
      {tags.map((tag) => `#${tag}`).join(" ")}
    </span>
  </li>
</a>
