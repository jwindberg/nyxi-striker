<script lang="ts">
  // Controls for one lighting zone. Reads the keyboard's current setting, and writes the
  // whole setting back on every change (debounced while sliders are dragged).
  import { onMount } from "svelte";
  import { api, COLOR_CUSTOM, COLOR_RAINBOW, SWATCHES, fromHex, toHex, type Lighting, type ModeInfo, type RGB, type Zone } from "./api";

  let { zone, title, note = "" }: { zone: Zone; title: string; note?: string } = $props();

  let modes = $state<ModeInfo[]>([]);
  let light = $state<Lighting | null>(null);
  let error = $state("");

  let mode = $derived(modes.find((m) => m.id === light?.mode));
  let isOff = $derived(light?.mode === 0);

  onMount(load);

  async function load() {
    error = "";
    try {
      modes = await api.modes(zone);
      light = await api.getLighting(zone);
    } catch (e) {
      error = String(e);
    }
  }

  let timer: ReturnType<typeof setTimeout> | undefined;
  function apply(wait = 0) {
    clearTimeout(timer);
    timer = setTimeout(async () => {
      if (!light) return;
      try {
        await api.setLighting(zone, light);
        error = "";
      } catch (e) {
        error = String(e);
      }
    }, wait);
  }

  function setMode(m: ModeInfo) {
    if (!light) return;
    light.mode = m.id;
    if (light.option >= m.options.length) light.option = 0;
    if (m.id !== 0 && light.brightness === 0) light.brightness = 4;
    apply();
  }

  function setColor(rgb: RGB | null) {
    if (!light) return;
    if (rgb) {
      light.color = COLOR_CUSTOM;
      light.rgb = rgb;
    } else {
      light.color = COLOR_RAINBOW;
    }
    apply();
  }

  const same = (a: RGB, b: RGB) => a.every((v, i) => v === b[i]);
</script>

<section class="panel">
  <header>
    <h2>{title}</h2>
    {#if note}<p class="note">{note}</p>{/if}
  </header>

  {#if error}
    <p class="error">{error} <button class="link" onclick={load}>Retry</button></p>
  {/if}

  {#if light}
    <div class="selects">
      <label class="field">
        <span class="label">Effect</span>
        <select value={light.mode} onchange={(e) => setMode(modes.find((m) => m.id === Number(e.currentTarget.value))!)}>
          {#each modes as m (m.id)}
            <option value={m.id}>{m.name}</option>
          {/each}
        </select>
      </label>

      {#if mode && mode.options.length}
        <label class="field">
          <span class="label">Variant</span>
          <select value={light.option} onchange={(e) => { light!.option = Number(e.currentTarget.value); apply(); }}>
            {#each mode.options as opt, i}
              <option value={i}>{opt}</option>
            {/each}
          </select>
        </label>
      {/if}
    </div>

    {#if !isOff}
      <div class="sliders">
        <label class="field">
          <span class="label">Brightness <b>{light.brightness}</b>/4</span>
          <input type="range" min="1" max="4" bind:value={light.brightness} oninput={() => apply(150)} />
        </label>
        {#if mode?.speed}
          <label class="field">
            <span class="label">Speed <b>{light.speed}</b>/4</span>
            <input type="range" min="0" max="4" bind:value={light.speed} oninput={() => apply(150)} />
          </label>
        {/if}
      </div>

      <div class="field">
        <span class="label">Color</span>
        <div class="swatches">
          <button class="swatch rainbow" class:active={light.color === COLOR_RAINBOW} title="Rainbow" aria-label="Rainbow" onclick={() => setColor(null)}></button>
          {#each SWATCHES as s}
            <button
              class="swatch"
              class:active={light.color === COLOR_CUSTOM && same(light.rgb, s.rgb)}
              style:background={toHex(s.rgb)}
              title={s.name}
              aria-label={s.name}
              onclick={() => setColor(s.rgb)}
            ></button>
          {/each}
          <label class="swatch picker" class:active={light.color === COLOR_CUSTOM && !SWATCHES.some((s) => same(light!.rgb, s.rgb))} title="Custom color">
            <input type="color" value={toHex(light.rgb)} oninput={(e) => { light!.color = COLOR_CUSTOM; light!.rgb = fromHex(e.currentTarget.value); apply(150); }} />
          </label>
        </div>
      </div>
    {/if}
  {:else if !error}
    <p class="muted">Reading the keyboard…</p>
  {/if}
</section>

<style>
  .panel {
    background: var(--panel);
    border: 1px solid var(--line);
    border-radius: 14px;
    padding: 18px 20px 20px;
  }
  header {
    margin-bottom: 14px;
  }
  h2 {
    margin: 0;
    font-size: 1.05rem;
    letter-spacing: 0.04em;
  }
  .note,
  .muted {
    margin: 4px 0 0;
    color: var(--muted);
    font-size: 0.85rem;
  }
  .error {
    color: var(--red-text);
    font-size: 0.9rem;
  }
  .field {
    display: block;
    margin-bottom: 14px;
  }
  .label {
    display: block;
    font-size: 0.78rem;
    text-transform: uppercase;
    letter-spacing: 0.08em;
    color: var(--muted);
    margin-bottom: 7px;
  }
  .label b {
    color: var(--text);
  }
  .selects {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(180px, 1fr));
    gap: 0 24px;
  }
  select {
    width: 100%;
    background: var(--key);
    color: var(--key-text);
    border: none;
    border-bottom: 3px solid var(--key-edge);
    border-radius: 7px;
    padding: 7px 10px;
    font: inherit;
    font-size: 0.92rem;
    cursor: pointer;
  }
  .sliders {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(180px, 1fr));
    gap: 0 24px;
  }
  input[type="range"] {
    width: 100%;
    accent-color: var(--accent);
  }
  .swatches {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
  }
  .swatch {
    width: 32px;
    height: 32px;
    border-radius: 50%;
    border: 2px solid rgba(255, 255, 255, 0.25);
    cursor: pointer;
    padding: 0;
    position: relative;
  }
  .swatch.active {
    outline: 3px solid var(--accent);
    outline-offset: 2px;
  }
  .rainbow {
    background: conic-gradient(red, orange, yellow, lime, cyan, blue, magenta, red);
  }
  .picker {
    background: conic-gradient(from 90deg, #888, #ddd, #888);
    overflow: hidden;
  }
  .picker input {
    position: absolute;
    inset: -6px;
    width: 44px;
    height: 44px;
    opacity: 0;
    cursor: pointer;
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
