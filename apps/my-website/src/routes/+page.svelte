<script lang="ts">
  import IntroSection from "./_index/components/IntroSection.svelte";
  import FindMeOnSection from "./_index/components/FindMeOnSection.svelte";
  import AboutMeSection from "./_index/components/AboutMeSection.svelte";
  import WhatImLearningSection from "./_index/components/WhatImLearningSection.svelte";
  import WhatImWorkingOnSection from "./_index/components/WhatImWorkingOnSection.svelte";
  import SeoMetaHeader from "$components/SEO/SEOMetaHeader.svelte";
</script>

<SeoMetaHeader
  siteTitle="Saikat's Website"
  siteDescription="This is a website where I blog about full stack JavaScript including things like SvelteJS, TailwindCSS, NodeJS, Linux, PostgreSQL among a few and showcase projects I've worked on" />

<main class="max-w-screen-md mx-auto p-2 mb-8">
  <IntroSection />
  <FindMeOnSection />
  <AboutMeSection />
  <WhatImLearningSection />
  <WhatImWorkingOnSection />
</main>
