<script lang="ts">
  import IndianFlagIcon from "$icons/twemoji/flag-for-flag-india.svelte";
  import LinkedinIcon from "$icons/logos/linkedin-icon.svelte";
  // import GmailIcon from "$icons/logos/google-gmail.svelte";
  import GithubIcon from "$icons/logos/github-icon.svelte";
</script>

<style>
  .wave {
    animation-name: wave-animation; /* Refers to the name of your @keyframes element below */
    animation-duration: 2.5s; /* Change to speed up or slow down */
    animation-iteration-count: infinite; /* Never stop waving :) */
    transform-origin: 70% 70%; /* Pivot around the bottom-left palm */
    display: inline-block;
  }

  @keyframes wave-animation {
    0% {
      transform: rotate(0deg);
    }
    10% {
      transform: rotate(14deg);
    } /* The following five values can be played with to make the waving more or less extreme */
    20% {
      transform: rotate(-8deg);
    }
    30% {
      transform: rotate(14deg);
    }
    40% {
      transform: rotate(-4deg);
    }
    50% {
      transform: rotate(10deg);
    }
    60% {
      transform: rotate(0deg);
    } /* Reset for the last half to pause */
    100% {
      transform: rotate(0deg);
    }
  }
</style>

<section class="text-xl mt-8 border-l-4 pl-8 border-fuchsia-500">
  <p>
    <span class="wave" role="img" aria-label="Hi">👋</span>
    , I'm Saikat!
    <br />
    I'm a programmer based out of
    <span role="img" aria-label="India" class="align-text-bottom">
      <IndianFlagIcon className="inline h-5" />
    </span>
    <br />
    I like
    <span role="img" aria-label="computers">🖥️</span>
    and
    <span role="img" aria-label="gaming">🎮</span>
    <br />
    Connect with me on
    <a
      href="https://www.linkedin.com/in/saikat-das-13674166/"
      target="_blank"
      rel="noopener noreferrer">
      <span role="img" aria-label="LinkedIn" class="align-text-bottom">
        <LinkedinIcon className="inline h-5" />
      </span>
    </a>
    or
    <a
      href="https://github.com/saikatdas0790"
      target="_blank"
      rel="noopener noreferrer">
      <span role="img" aria-label="github" class="align-text-bottom">
        <GithubIcon className="inline h-5" />
      </span>
    </a>
    <!-- <br />
    You can also{" "}
    <a
      href="mailto:saikatdas0790@gmail.com"
      target="_blank"
      rel="noopener noreferrer">
      <span role="img" aria-label="email" class="align-text-bottom">
        <GmailIcon className="inline h-5" />
      </span>
    </a>
    {" "}
    me in case you want to talk.
  </p> -->
  </p>
</section>
