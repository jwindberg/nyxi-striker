<script lang="ts">
  // Paints one of the keyboard's three per-key pictures. The backlight must already be showing
  // that picture: the keyboard reads and uploads only the picture it shows. Every finished
  // stroke or fill uploads the whole picture.
  import { onMount } from "svelte";
  import { api, SWATCHES, fromHex, toHex, type Key, type Picture, type RGB } from "./api";

  let { picture, busy = $bindable(false) }: { picture: number; busy?: boolean } = $props();

  const OFF: RGB = [0, 0, 0];
  // Gradients laid across the keyboard from left to right.
  const PALETTES: { name: string; stops: RGB[] }[] = [
    { name: "Rainbow", stops: [[255, 0, 0], [255, 255, 0], [0, 255, 0], [0, 255, 255], [0, 0, 255], [255, 0, 255]] },
    { name: "Sunset", stops: [[255, 40, 0], [255, 120, 0], [255, 0, 120], [120, 0, 255]] },
    { name: "Ocean", stops: [[0, 40, 255], [0, 160, 255], [0, 255, 200]] },
    { name: "Forest", stops: [[0, 255, 40], [120, 255, 0], [0, 120, 20]] },
    { name: "Fire", stops: [[255, 0, 0], [255, 80, 0], [255, 200, 0]] },
    { name: "Ice", stops: [[255, 255, 255], [120, 200, 255], [0, 80, 255]] },
  ];

  let keys = $state<Key[]>([]);
  let colors = $state<Picture | null>(null);
  let brush = $state<RGB>([255, 255, 255]);
  let error = $state("");
  let painting = false;
  let dirty = false;

  let width = $derived(Math.max(0, ...keys.map((k) => k.x + k.w)));
  let height = $derived(Math.max(0, ...keys.map((k) => k.y + k.h)));

  onMount(async () => {
    try {
      keys = await api.keys();
      colors = await api.getPicture(picture);
    } catch (e) {
      error = String(e);
    }
  });

  // One upload at a time; changes made during an upload go up right after it.
  async function save() {
    dirty = true;
    if (busy || !colors) return;
    busy = true;
    try {
      while (dirty) {
        dirty = false;
        await api.setPicture(picture, $state.snapshot(colors) as Picture);
      }
      error = "";
    } catch (e) {
      error = String(e);
    } finally {
      busy = false;
    }
  }

  function paint(k: Key) {
    if (!colors || same(colors[k.slot], brush)) return;
    colors[k.slot] = [...brush];
  }

  function stopPainting() {
    if (!painting) return;
    painting = false;
    save();
  }

  function fill(color: (k: Key) => RGB) {
    if (!colors) return;
    for (const k of keys) colors[k.slot] = color(k);
    save();
  }

  function gradient(stops: RGB[]) {
    fill((k) => {
      const t = ((k.x + k.w / 2) / width) * (stops.length - 1);
      const i = Math.min(Math.floor(t), stops.length - 2);
      const f = t - i;
      return stops[i].map((v, c) => Math.round(v + (stops[i + 1][c] - v) * f)) as RGB;
    });
  }

  const same = (a: RGB, b: RGB) => a.every((v, i) => v === b[i]);
  const css = (stops: RGB[]) => `linear-gradient(90deg, ${stops.map(toHex).join(", ")})`;
  // Dark text on bright keys, light text on dark ones.
  const ink = ([r, g, b]: RGB) => (0.299 * r + 0.587 * g + 0.114 * b > 140 ? "#23243f" : "#eef0ff");
</script>

<svelte:window onpointerup={stopPainting} />

{#if error}<p class="error">{error}</p>{/if}

{#if colors && keys.length}
  <div class="field">
    <span class="label">Keys <b>· click or drag to paint</b></span>
    <svg viewBox="-2 -2 {width + 4} {height + 4}" class:busy role="img" aria-label="Keyboard key colors">
      {#each keys as k (k.slot)}
        {@const c = colors[k.slot]}
        <g
          role="button"
          tabindex="-1"
          aria-label={k.label || "Space"}
          onpointerdown={(e) => { e.preventDefault(); painting = true; paint(k); }}
          onpointerenter={() => painting && paint(k)}
        >
          <rect x={k.x} y={k.y} width={k.w} height={k.h} rx="4" fill={same(c, OFF) ? "#191a3d" : toHex(c)} />
          <text x={k.x + k.w / 2} y={k.y + k.h / 2} fill={same(c, OFF) ? "#6d70a8" : ink(c)}>{k.label}</text>
        </g>
      {/each}
    </svg>
  </div>

  <div class="field">
    <span class="label">Paint color</span>
    <div class="swatches">
      <button class="swatch off" class:active={same(brush, OFF)} title="Off (key unlit)" aria-label="Off (key unlit)" onclick={() => (brush = OFF)}></button>
      {#each SWATCHES as s}
        <button class="swatch" class:active={same(brush, s.rgb)} style:background={toHex(s.rgb)} title={s.name} aria-label={s.name} onclick={() => (brush = s.rgb)}></button>
      {/each}
      <label class="swatch picker" class:active={!same(brush, OFF) && !SWATCHES.some((s) => same(brush, s.rgb))} title="Custom color">
        <input type="color" value={toHex(brush)} oninput={(e) => (brush = fromHex(e.currentTarget.value))} />
      </label>
    </div>
  </div>

  <div class="field">
    <span class="label">Fill every key</span>
    <div class="fills">
      <button class="key" onclick={() => fill(() => [...brush])}>With paint color</button>
      {#each PALETTES as p}
        <button class="key palette" onclick={() => gradient(p.stops)}><span style:background={css(p.stops)}></span>{p.name}</button>
      {/each}
      <button class="key" onclick={() => fill(() => OFF)}>All off</button>
    </div>
  </div>
{:else if !error}
  <p class="note">Reading the picture…</p>
{/if}

<style>
  svg {
    display: block;
    width: 100%;
    height: auto;
    touch-action: none;
    user-select: none;
  }
  svg.busy {
    cursor: progress;
  }
  rect {
    stroke: var(--line);
    stroke-width: 1;
    cursor: pointer;
  }
  g:hover rect {
    stroke: var(--accent);
    stroke-width: 2;
  }
  text {
    font-size: 8px;
    font-weight: 600;
    text-anchor: middle;
    dominant-baseline: central;
    pointer-events: none;
  }
  .off {
    background: repeating-linear-gradient(45deg, #191a3d 0 4px, #2a2c72 4px 8px);
  }
  .fills {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
  }
  .palette {
    display: inline-flex;
    align-items: center;
    gap: 7px;
  }
  .palette span {
    width: 28px;
    height: 12px;
    border-radius: 3px;
  }
</style>
