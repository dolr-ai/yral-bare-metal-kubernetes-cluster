<script lang="ts">
  import { format } from "date-fns";

  export let title: string;
  export let description: string;
  export let technologiesUsed: string[];
  export let startDate: Date;
  export let endDate: Date;
  export let coverPhoto: string;
  export let slug: string;
</script>

<style>
  @keyframes rotate {
    to {
      --angle: 360deg;
    }
  }

  li:hover {
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
</style>

<li class="border-4 rounded-lg border-transparent block my-2 p-2 md:py-6">
  <a href={slug} data-sveltekit-preload-data="hover">
    <img
      src={`/assets/images/routes/projects/entries/${coverPhoto}`}
      alt="Large screenshot of landing page"
      width="1280"
      height="720"
      loading="lazy" />
    <h2 class="text-2xl font-medium my-2">{title}</h2>
    <p>
      worked on this between <span class="font-semibold">
        {format(new Date(startDate), "MMM yyyy")}
      </span>
      and
      <span class="font-semibold">{format(new Date(endDate), "MMM yyyy")}</span>
    </p>
    <p class="my-4">{description}</p>
    <p class="text-fuchsia-600 text-sm">#{technologiesUsed.join(" #")}</p>
  </a>
</li>
