<script lang="ts">
  import { onMount } from "svelte";
  import DotScreen from "$lib/DotScreen.svelte";
  import LightingPanel from "$lib/LightingPanel.svelte";
  import ScreenPanel from "$lib/ScreenPanel.svelte";
  import { api, type Frame, type Info } from "$lib/api";

  let tab = $state<"lighting" | "screen">("lighting");
  let info = $state<Info | null>(null);
  let connectError = $state("");
  // What we last sent to the screen; drawn over the screen in the keyboard picture.
  let shown = $state<{ frames: Frame[]; delay: number } | null>(null);

  onMount(connect);

  async function connect() {
    try {
      info = await api.info();
      connectError = "";
    } catch (e) {
      info = null;
      connectError = String(e);
    }
  }

  const version = (v: number) => `${v >> 8}.${String(v & 0xff).padStart(2, "0")}`;
</script>

<main>
  <div class="hero">
    <div class="keyboard">
      <img src="/img/ca01.png" alt="NYXI CA01 Striker keyboard" />
      {#if shown}
        <div class="overlay"><DotScreen frames={shown.frames} delay={shown.delay} /></div>
      {/if}
    </div>
    <div class="status">
      {#if info}
        <span class="dot ok"></span>
        CA01 connected · {info.wireless ? "2.4 GHz" : "wired"} · firmware {version(info.firmware)}
      {:else if connectError}
        <span class="dot bad"></span>
        {connectError}
        <button class="link" onclick={connect}>Retry</button>
      {:else}
        Looking for the keyboard…
      {/if}
    </div>
  </div>

  <nav>
    <button class:active={tab === "lighting"} onclick={() => (tab = "lighting")}>Lighting</button>
    <button class:active={tab === "screen"} onclick={() => (tab = "screen")}>Screen</button>
  </nav>

  {#if tab === "lighting"}
    <div class="stack">
      <LightingPanel zone="main" title="Backlight" />
      <LightingPanel zone="side" title="Second zone" note="The keyboard's second lighting setting. Which lights it drives on the CA01 is still unknown." />
    </div>
  {:else}
    <ScreenPanel onshow={(frames, delay) => (shown = { frames, delay })} />
  {/if}
</main>

<style>
  :global(:root) {
    --bg: #15163a;
    --panel: #20225a;
    --panel-2: #2a2c72;
    --line: #3a3d8f;
    --text: #eef0ff;
    --muted: #a6a9d6;
    --accent: #f5c400;
    --accent-edge: #b88f00;
    --key: #d7d9e2;
    --key-edge: #a4a7b4;
    --key-text: #23243f;
    --red-text: #ff8a8f;
    --screen: #414243;
    color-scheme: dark;
  }
  :global(body) {
    margin: 0;
    background: var(--bg);
    color: var(--text);
    font-family: system-ui, -apple-system, "Segoe UI", Roboto, sans-serif;
  }
  main {
    max-width: 980px;
    margin: 0 auto;
    padding: 18px 20px 32px;
  }
  .hero {
    margin-bottom: 14px;
  }
  .keyboard {
    position: relative;
  }
  .keyboard img {
    display: block;
    width: 100%;
    height: auto;
  }
  /* The screen in ca01.png (871x288): a 9x9 grid of dots, 5 px apart, from (732.5, 20). */
  .overlay {
    position: absolute;
    left: calc(728 / 871 * 100%);
    top: calc(15 / 288 * 100%);
    width: calc(49 / 871 * 100%);
    height: calc(52 / 288 * 100%);
    background: var(--screen);
    padding: 0.35% 0.25%;
    box-sizing: border-box;
  }
  .status {
    display: flex;
    align-items: center;
    gap: 8px;
    justify-content: center;
    color: var(--muted);
    font-size: 0.88rem;
    margin-top: 6px;
  }
  .dot {
    width: 9px;
    height: 9px;
    border-radius: 50%;
  }
  .ok {
    background: #48d16b;
  }
  .bad {
    background: #e5484d;
  }
  nav {
    display: flex;
    gap: 6px;
    margin-bottom: 14px;
    border-bottom: 1px solid var(--line);
  }
  nav button {
    background: none;
    border: none;
    border-bottom: 3px solid transparent;
    color: var(--muted);
    font: inherit;
    font-size: 0.98rem;
    padding: 8px 14px;
    cursor: pointer;
  }
  nav button.active {
    color: var(--text);
    border-bottom-color: var(--accent);
  }
  .stack {
    display: grid;
    gap: 14px;
  }
  .link {
    background: none;
    border: none;
    color: var(--accent);
    cursor: pointer;
    font: inherit;
    text-decoration: underline;
  }
</style>
