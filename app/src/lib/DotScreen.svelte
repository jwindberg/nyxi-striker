<script lang="ts">
  // A 9x9 dot display that plays frames at the keyboard's speed (delay x ~20 ms).
  import { DELAY_UNIT_MS, type Frame } from "./api";

  let { frames, delay = 50, playing = true }: { frames: Frame[]; delay?: number; playing?: boolean } = $props();

  let index = $state(0);

  $effect(() => {
    index = 0;
    if (!playing || frames.length < 2) return;
    const timer = setInterval(() => (index = (index + 1) % frames.length), Math.max(1, delay) * DELAY_UNIT_MS);
    return () => clearInterval(timer);
  });

  let frame = $derived(frames[index % frames.length] ?? []);
</script>

<div class="dots">
  {#each frame as row}
    {#each row as on}
      <span class:on></span>
    {/each}
  {/each}
</div>

<style>
  .dots {
    display: grid;
    grid-template-columns: repeat(9, 1fr);
    grid-template-rows: repeat(9, 1fr);
    width: 100%;
    height: 100%;
  }
  span {
    margin: 14%;
    border-radius: 50%;
    background: var(--dot-off, rgba(255, 255, 255, 0.07));
  }
  span.on {
    background: var(--dot-on, #f4f6ff);
    box-shadow: 0 0 3px var(--dot-glow, rgba(255, 255, 255, 0.7));
  }
</style>
