<script lang="ts">
  export let menuOpen: boolean = false;
</script>

<style>
  button > div {
    width: 60%;
    height: 100%;
    position: relative;
    transform: rotate(0deg);
    cursor: pointer;
    margin: auto;
  }

  button > div > span {
    display: block;
    position: absolute;
    height: 10%;
    width: 100%;
    background: white;
    border-radius: 10%;
    opacity: 1;
    left: 0;
    transform: rotate(0deg);
    transform: translateY(-50%);
    transition: transform 0.5s ease-in-out, top 0.5s ease-in-out,
      opacity 0.5s ease-in-out;
    will-change: transform, top, opacity;
  }

  button > div > span:nth-child(1) {
    top: 25%;
  }

  button > div > span:nth-child(2) {
    top: 50%;
  }

  button > div > span:nth-child(3) {
    top: 75%;
  }

  button > div.open > span:nth-child(1) {
    top: 45%;
    transform: translateY(-50%);
    transform: rotate(135deg);
  }

  button > div.open > span:nth-child(2) {
    transform: translateX(-200%);
    opacity: 0;
  }

  button > div.open > span:nth-child(3) {
    top: 45%;
    transform: translateY(-50%);
    transform: rotate(-135deg);
  }
</style>

<button
  aria-label="Toggle navigation menu"
  class="w-12 h-12 fixed bottom-8 right-8 bg-gradient-to-tr from-emerald-500 via-blue-500 to-fuchsia-500 rounded-lg focus:outline-none focus:ring focus:ring-emerald-300 shadow-lg z-10"
  on:click={() => (menuOpen = !menuOpen)}>
  <div class:open={menuOpen}>
    <span></span>
    <span></span>
    <span></span>
  </div>
</button>
