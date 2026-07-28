<script>
  import SvelteIcon from "$icons/logos/svelte-icon.svelte";
  import GmailIcon from "$icons/logos/google-gmail.svelte";
  import LinkedInIcon from "$icons/logos/linkedin-icon.svelte";
  import GithubIcon from "$icons/logos/github-icon.svelte";
  import NPMIcon from "$icons/logos/npm-icon.svelte";
  import StackOverflowIcon from "$icons/logos/stackoverflow-icon.svelte";
</script>

<style>
  h2 {
    text-decoration-color: #6ee7b7;
    text-decoration-line: underline overline;
    text-underline-offset: 0.75rem;
    text-decoration-style: wavy;
  }
</style>

<h2 class="text-3xl my-8 mt-16">Find me at</h2>
<article>
  <ul class="grid grid-cols-2 md:grid-cols-3">
    <li class="mb-4">
      <a
        href="https://saikat.dev"
        target="_blank"
        class="flex gap-2 items-center text-xl p-4 border-l-2 border-yellow-600 hover:shadow-md transition duration-300">
        <SvelteIcon className="w-5 h-5" />
        <span>My Website</span>
      </a>
    </li>
    <li class="mb-4">
      <a
        href="https://github.com/saikatdas0790"
        target="_blank"
        class="flex gap-2 items-center text-xl p-4 border-l-2 border-black hover:shadow-md transition duration-300">
        <GithubIcon className="w-5 h-5" />
        <span>Github</span>
      </a>
    </li>
    <li class="mb-4">
      <a
        href="https://www.npmjs.com/~saikatdas0790"
        target="_blank"
        class="flex gap-2 items-center text-xl p-4 border-l-2 border-red-600 hover:shadow-md transition duration-300">
        <NPMIcon className="w-5 h-5" />
        <span>NPM</span>
      </a>
    </li>
    <li class="mb-4">
      <a
        href="mailto:saikatdas0790@gmail.com"
        target="_blank"
        class="flex gap-2 items-center text-xl p-4 border-l-2 border-red-500 hover:shadow-md transition duration-300">
        <GmailIcon className="w-5 h-5" />
        <span>Email</span>
      </a>
    </li>
    <li class="mb-4">
      <a
        href="https://www.linkedin.com/in/saikat-das-13674166/"
        target="_blank"
        class="flex gap-2 items-center text-xl p-4 border-l-2 border-blue-700 hover:shadow-md transition duration-300">
        <LinkedInIcon className="w-5 h-5" />
        <span>LinkedIn</span>
      </a>
    </li>
    <li class="mb-4">
      <a
        href="https://stackoverflow.com/users/3462837/saikat-das"
        target="_blank"
        class="flex gap-2 items-center text-xl p-4 border-l-2 border-yellow-500 hover:shadow-md transition duration-300">
        <StackOverflowIcon className="w-5 h-5" />
        <span>Stack Overflow</span>
      </a>
    </li>
  </ul>
</article>
