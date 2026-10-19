<script lang="ts">
  import { onMount } from "svelte";
  import DotScreen from "$lib/DotScreen.svelte";
  import LightingPanel from "$lib/LightingPanel.svelte";
  import ScreenPanel from "$lib/ScreenPanel.svelte";
  import SettingsPanel from "$lib/SettingsPanel.svelte";
  import { api, type Frame, type Info } from "$lib/api";

  let tab = $state<"lighting" | "screen" | "settings">("lighting");
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
  <header>
    <div class="keyboard">
      <img src="/img/ca01.png" alt="NYXI CA01 Striker keyboard" />
      {#if shown}
        <div class="overlay"><DotScreen frames={shown.frames} delay={shown.delay} /></div>
      {/if}
    </div>
    <div class="status">
      <h1>CA01 Striker</h1>
      {#if info}
        <p><span class="dot ok"></span>Connected · {info.wireless ? "2.4 GHz" : "wired"} · firmware {version(info.firmware)}</p>
      {:else if connectError}
        <p><span class="dot bad"></span>{connectError}</p>
        <button class="link" onclick={connect}>Retry</button>
      {:else}
        <p>Looking for the keyboard…</p>
      {/if}
    </div>
  </header>

  <nav>
    <button class:active={tab === "lighting"} onclick={() => (tab = "lighting")}>Lighting</button>
    <button class:active={tab === "screen"} onclick={() => (tab = "screen")}>Screen</button>
    <button class:active={tab === "settings"} onclick={() => (tab = "settings")}>Settings</button>
  </nav>

  {#if tab === "lighting"}
    <!-- The firmware also has a second lighting zone (zone="side"), but nothing on the CA01 lights up for it. -->
    <LightingPanel zone="main" title="Backlight" />
  {:else if tab === "screen"}
    <ScreenPanel onshow={(frames, delay) => (shown = { frames, delay })} />
  {:else}
    <SettingsPanel {info} />
  {/if}
</main>

<style>
  main {
    max-width: 980px;
    margin: 0 auto;
    padding: 16px 20px 32px;
  }
  header {
    display: flex;
    align-items: center;
    gap: 22px;
    margin-bottom: 10px;
  }
  .keyboard {
    position: relative;
    width: 420px;
    flex-shrink: 0;
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
  h1 {
    margin: 0 0 6px;
    font-size: 1.3rem;
    letter-spacing: 0.05em;
  }
  .status p {
    display: flex;
    align-items: center;
    gap: 8px;
    margin: 0;
    color: var(--muted);
    font-size: 0.88rem;
  }
  .dot {
    width: 9px;
    height: 9px;
    border-radius: 50%;
    flex-shrink: 0;
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
</style>
