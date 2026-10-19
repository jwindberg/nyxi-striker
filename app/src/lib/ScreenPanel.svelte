<script lang="ts">
  // The image and animation library: everything previews live; choosing one uploads it to
  // the keyboard's custom slot, which the screen switches to immediately. An image is
  // simply a one-frame animation.
  import { onMount } from "svelte";
  import DotScreen from "./DotScreen.svelte";
  import { api, blankFrame, DELAY_UNIT_MS, type Animation, type Frame } from "./api";

  let { onshow }: { onshow: (frames: Frame[], delay: number) => void } = $props();

  let library = $state<Animation[]>([]);
  let selected = $state<Animation | null>(null);
  let delay = $state(50);
  let busy = $state(false);
  let error = $state("");

  let fps = $derived((1000 / (Math.max(1, delay) * DELAY_UNIT_MS)).toFixed(1));
  let images = $derived(library.filter((a) => a.frames.length === 1));
  let animations = $derived(library.filter((a) => a.frames.length > 1));
  let animated = $derived((selected?.frames.length ?? 0) > 1);

  onMount(async () => {
    library = await api.animations();
  });

  async function send(frames: Frame[], d: number) {
    busy = true;
    try {
      await api.play(frames, d);
      error = "";
      onshow(frames, d);
    } catch (e) {
      error = String(e);
    } finally {
      busy = false;
    }
  }

  function choose(a: Animation) {
    selected = a;
    delay = a.delay;
    send(a.frames, a.delay);
  }

  let timer: ReturnType<typeof setTimeout> | undefined;
  function changeSpeed() {
    clearTimeout(timer);
    if (selected) timer = setTimeout(() => send(selected!.frames, delay), 400);
  }

  function clear() {
    selected = null;
    send([blankFrame()], 0);
  }
</script>

<section class="panel">
  <header>
    <div>
      <h2>Screen</h2>
      <p class="note">Choosing an image or animation shows it on the keyboard right away, in its custom slot.</p>
    </div>
    <button class="key" onclick={clear} disabled={busy}>Clear screen</button>
  </header>


  {#if error}<p class="error">{error}</p>{/if}

  <!-- Always present so the grid below never shifts; speed only applies to animations. -->
  <div class="now">
    <span class="label">
      {#if selected}{animated ? "Playing" : "Showing"} <b>{selected.name}</b>{:else}Nothing chosen yet{/if}
    </span>
    <label class="speed" class:inactive={!animated}>
      <span class="label">Speed <b>{animated ? fps : "–"}</b> frames/s</span>
      <!-- Slider runs fast (left = slow) by inverting the delay. -->
      <input type="range" min="-60" max="-2" value={-delay} disabled={!animated} oninput={(e) => { delay = -Number(e.currentTarget.value); changeSpeed(); }} />
    </label>
  </div>

  {#snippet grid(items: Animation[])}
    <div class="grid">
      {#each items as a (a.slug)}
        <button class="card" class:active={selected?.slug === a.slug} onclick={() => choose(a)} title={a.about} disabled={busy}>
          <div class="screen"><DotScreen frames={a.frames} delay={a.delay} /></div>
          <span class="name">{a.name}</span>
        </button>
      {/each}
    </div>
  {/snippet}

  <h3>Images</h3>
  {@render grid(images)}
  <h3>Animations</h3>
  {@render grid(animations)}
</section>

<style>
  header {
    display: flex;
    gap: 16px;
    justify-content: space-between;
    align-items: flex-start;
    margin-bottom: 14px;
  }
  .now {
    display: grid;
    grid-template-columns: 1fr minmax(200px, 1.4fr);
    gap: 24px;
    align-items: end;
    background: var(--panel-2);
    border-radius: 10px;
    padding: 12px 14px 4px;
    margin-bottom: 16px;
  }
  h3 {
    margin: 18px 0 10px;
    font-size: 0.78rem;
    text-transform: uppercase;
    letter-spacing: 0.08em;
    color: var(--muted);
    font-weight: 600;
  }
  .grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(104px, 1fr));
    gap: 12px;
  }
  .card {
    background: var(--panel-2);
    border: 2px solid transparent;
    border-radius: 12px;
    padding: 10px 10px 8px;
    color: var(--text);
    font: inherit;
    cursor: pointer;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 7px;
  }
  .card:hover {
    border-color: var(--line);
  }
  .card.active {
    border-color: var(--accent);
  }
  .card:disabled {
    cursor: progress;
  }
  .screen {
    width: 100%;
    aspect-ratio: 1;
    background: var(--screen);
    border-radius: 8px;
    padding: 6%;
  }
  .name {
    font-size: 0.82rem;
    text-align: center;
  }
</style>
