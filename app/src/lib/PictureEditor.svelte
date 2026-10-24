<script lang="ts">
  // Paints one of the keyboard's three per-key pictures. The backlight must already be showing
  // that picture: the keyboard reads and uploads only the picture it shows. Every finished
  // stroke or fill uploads the whole picture. Pictures can also be saved on this computer, as
  // text files in the pictures folder, and put back on the keyboard later.
  import { onMount } from "svelte";
  import { api, SWATCHES, fromHex, toHex, type Key, type Picture, type RGB, type SavedPicture } from "./api";

  let { picture, busy = $bindable(false) }: { picture: number; busy?: boolean } = $props();

  const OFF: RGB = [0, 0, 0];
  // Gradients laid across the keyboard in the chosen direction.
  const PALETTES: { name: string; stops: RGB[] }[] = [
    { name: "Rainbow", stops: [[255, 0, 0], [255, 255, 0], [0, 255, 0], [0, 255, 255], [0, 0, 255], [255, 0, 255]] },
    { name: "Sunset", stops: [[255, 40, 0], [255, 120, 0], [255, 0, 120], [120, 0, 255]] },
    { name: "Ocean", stops: [[0, 40, 255], [0, 160, 255], [0, 255, 200]] },
    { name: "Forest", stops: [[0, 255, 40], [120, 255, 0], [0, 120, 20]] },
    { name: "Fire", stops: [[255, 0, 0], [255, 80, 0], [255, 200, 0]] },
    { name: "Ice", stops: [[255, 255, 255], [120, 200, 255], [0, 80, 255]] },
  ];

  // Where each key sits along the gradient, 0 at its start and 1 at its end.
  const DIRECTIONS: { name: string; at: (x: number, y: number) => number }[] = [
    { name: "Left to right", at: (x) => x },
    { name: "Right to left", at: (x) => 1 - x },
    { name: "Top to bottom", at: (_, y) => y },
    { name: "Bottom to top", at: (_, y) => 1 - y },
    { name: "Diagonal", at: (x, y) => (x + y) / 2 },
  ];

  let keys = $state<Key[]>([]);
  let colors = $state<Picture | null>(null);
  let brush = $state<RGB>([255, 255, 255]);
  let direction = $state(0);
  let saved = $state<SavedPicture[]>([]);
  let saveName = $state("");
  let confirmDelete = $state<string | null>(null);
  let folder = $state("");
  let error = $state("");
  let painting = false;
  let dirty = false;

  let width = $derived(Math.max(0, ...keys.map((k) => k.x + k.w)));
  let height = $derived(Math.max(0, ...keys.map((k) => k.y + k.h)));

  onMount(async () => {
    try {
      keys = await api.keys();
      colors = await api.getPicture(picture);
      saved = await api.savedPictures();
      folder = await api.picturesFolder().catch(() => "");
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
    const at = DIRECTIONS[direction].at;
    fill((k) => {
      const t = at((k.x + k.w / 2) / width, (k.y + k.h / 2) / height) * (stops.length - 1);
      const i = Math.min(Math.floor(t), stops.length - 2);
      const f = t - i;
      return stops[i].map((v, c) => Math.round(v + (stops[i + 1][c] - v) * f)) as RGB;
    });
  }

  // Saving under an existing name replaces that saved picture.
  let replacing = $derived(saved.find((p) => p.name.toLowerCase() === saveName.trim().toLowerCase()));

  async function savePicture() {
    if (!colors || !saveName.trim()) return;
    try {
      const p = await api.savePicture(replacing?.slug ?? null, saveName, $state.snapshot(colors) as Picture);
      saved = [...saved.filter((s) => s.slug !== p.slug), p].sort((a, b) => a.slug.localeCompare(b.slug));
      saveName = "";
      error = "";
    } catch (e) {
      error = String(e);
    }
  }

  function load(p: SavedPicture) {
    if (!colors) return;
    colors = p.colors.map((c) => [...c] as RGB);
    save();
  }

  async function remove(slug: string) {
    try {
      await api.deletePicture(slug);
      saved = saved.filter((p) => p.slug !== slug);
      error = "";
    } catch (e) {
      error = String(e);
    }
    confirmDelete = null;
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
    <label class="direction">
      Palettes run
      <select bind:value={direction}>
        {#each DIRECTIONS as d, i}
          <option value={i}>{d.name.toLowerCase()}</option>
        {/each}
      </select>
    </label>
    <div class="fills">
      <button class="key" onclick={() => fill(() => [...brush])}>With paint color</button>
      {#each PALETTES as p}
        <button class="key palette" onclick={() => gradient(p.stops)}><span style:background={css(p.stops)}></span>{p.name}</button>
      {/each}
      <button class="key" onclick={() => fill(() => OFF)}>All off</button>
    </div>
  </div>
  <div class="field">
    <span class="label">Saved on this computer</span>
    {#if saved.length}
      <div class="saved">
        {#each saved as p (p.slug)}
          <div class="card">
            {#if confirmDelete === p.slug}
              <p>Delete “{p.name}”?</p>
              <div class="row">
                <button class="key" onclick={() => remove(p.slug)}>Delete</button>
                <button class="key" onclick={() => (confirmDelete = null)}>Keep</button>
              </div>
            {:else}
              <button class="thumb" title="Put “{p.name}” on the keyboard" onclick={() => load(p)}>
                <svg viewBox="0 0 {width} {height}" aria-hidden="true">
                  {#each keys as k (k.slot)}
                    <rect x={k.x} y={k.y} width={k.w} height={k.h} rx="3" fill={same(p.colors[k.slot], OFF) ? "#191a3d" : toHex(p.colors[k.slot])} />
                  {/each}
                </svg>
                <span>{p.name}</span>
              </button>
              <button class="remove" title="Delete “{p.name}”" aria-label="Delete {p.name}" onclick={() => (confirmDelete = p.slug)}>×</button>
            {/if}
          </div>
        {/each}
      </div>
      <p class="note">Click one to put it on the keyboard as picture {picture + 1}.</p>
    {/if}
    <form class="row save" onsubmit={(e) => { e.preventDefault(); savePicture(); }}>
      <input type="text" placeholder="Name" maxlength="40" bind:value={saveName} />
      <button class="key" disabled={!saveName.trim()}>{replacing ? `Replace “${replacing.name}”` : "Save this picture"}</button>
    </form>
    {#if folder}<p class="note">Saved as text files in <code>{folder}</code>; copy them there to use them on another computer.</p>{/if}
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
  .direction {
    display: flex;
    align-items: center;
    gap: 8px;
    margin-bottom: 10px;
    font-size: 0.88rem;
    color: var(--muted);
  }
  .direction select {
    width: auto;
  }
  .saved {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(170px, 1fr));
    gap: 10px;
  }
  .card {
    position: relative;
    background: var(--panel-2);
    border: 1px solid var(--line);
    border-radius: 10px;
    min-height: 92px;
  }
  .card p {
    margin: 12px 10px 8px;
    font-size: 0.88rem;
  }
  .card .row {
    margin: 0 10px 10px;
  }
  .thumb {
    display: block;
    width: 100%;
    background: none;
    border: none;
    color: var(--text);
    font: inherit;
    font-size: 0.85rem;
    padding: 8px 8px 7px;
    cursor: pointer;
    text-align: left;
  }
  .thumb svg rect {
    stroke: none;
    cursor: inherit;
  }
  .thumb span {
    display: block;
    margin-top: 5px;
    padding-right: 18px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .card:hover {
    border-color: var(--accent);
  }
  .remove {
    position: absolute;
    right: 4px;
    bottom: 3px;
    background: none;
    border: none;
    color: var(--muted);
    font-size: 1.1rem;
    cursor: pointer;
    padding: 2px 6px;
  }
  .remove:hover {
    color: var(--red-text);
  }
  .row {
    display: flex;
    gap: 8px;
    align-items: center;
  }
  .save {
    margin-top: 10px;
  }
  .save input {
    width: 220px;
    background: var(--panel-2);
    color: var(--text);
    border: 1px solid var(--line);
    border-radius: 8px;
    padding: 7px 10px;
    font: inherit;
    font-size: 0.88rem;
  }
  .key:disabled {
    opacity: 0.5;
    cursor: default;
  }
  code {
    font-size: 0.8rem;
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
