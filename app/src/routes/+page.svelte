<script lang="ts">
  import { onMount } from "svelte";
  import KeyboardArt from "$lib/KeyboardArt.svelte";
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
    <div class="keyboard"><KeyboardArt {shown} /></div>
    <div class="status">
      <h1>Striker</h1>
      {#if info}
        <p><span class="dot ok"></span>Connected · {info.wireless ? "2.4 GHz" : "wired"} · firmware {version(info.firmware)}</p>
      {:else if connectError}
        <p><span class="dot bad"></span>{connectError}</p>
        <button class="link" onclick={connect}>Retry</button>
      {:else}
        <p>Looking for the keyboard…</p>
      {/if}
      {#if tab === "screen"}
        <div class="shortcuts">
          <p>The keyboard's own animations, from the keyboard:</p>
          <p><kbd>Fn</kbd> + <kbd>Left Ctrl</kbd> cycles jumping bird → custom slot → Pac-Man → cat</p>
          <p><kbd>Fn</kbd> + <kbd>Page Down</kbd> turns the screen on or off</p>
        </div>
      {/if}
    </div>
  </header>

  <nav>
    <button class:active={tab === "lighting"} onclick={() => (tab = "lighting")}>Lighting</button>
    <button class:active={tab === "screen"} onclick={() => (tab = "screen")}>Screen</button>
    <button class:active={tab === "settings"} onclick={() => (tab = "settings")}>Settings</button>
  </nav>

  {#if tab === "lighting"}
    <!-- The firmware also has a second lighting zone (zone="side"), but nothing on the Striker lights up for it. -->
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
    width: 420px;
    flex-shrink: 0;
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
  .shortcuts {
    margin-top: 10px;
    padding-left: 10px;
    border-left: 3px solid var(--accent);
  }
  .shortcuts p {
    display: block;
    margin: 0 0 3px;
    font-size: 0.82rem;
    line-height: 1.5;
  }
  kbd {
    background: var(--key);
    color: var(--key-text);
    border-bottom: 2px solid var(--key-edge);
    border-radius: 4px;
    padding: 0 5px;
    font: inherit;
    font-size: 0.75rem;
    font-weight: 600;
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
