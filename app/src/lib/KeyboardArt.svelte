<script lang="ts">
  // The Striker drawn from its key layout, colored like the real keyboard, with its 9x9 screen in
  // the top-right corner showing what was last sent to it.
  import { onMount } from "svelte";
  import DotScreen from "./DotScreen.svelte";
  import { api, type Frame, type Key } from "./api";

  let { shown = null }: { shown?: { frames: Frame[]; delay: number } | null } = $props();

  const PAD = 12;
  // The screen sits above the numpad, right of Page Down, partly above the function row.
  const SCREEN = 46;
  const TOP = 18;
  const GRAY = new Set(["Tab", "Caps", "Shift", "Ctrl", "Win", "Alt", "Backspace", "Enter", "Num", "*", "+", "Ent"]);
  const ARROWS = new Set(["↑", "←", "↓", "→"]);

  let keys = $state<Key[]>([]);
  let width = $derived(Math.max(0, ...keys.map((k) => k.x + k.w)));
  let height = $derived(Math.max(0, ...keys.map((k) => k.y + k.h)));
  let numpadX = $derived(keys.find((k) => k.label === "Num")?.x ?? Infinity);

  onMount(async () => {
    keys = await api.keys().catch(() => []);
  });

  function fill(k: Key) {
    if (k.label === "Esc") return "#d63a45";
    if (ARROWS.has(k.label)) return "#f5c400";
    if (GRAY.has(k.label) || (k.x >= numpadX && ["/", "-"].includes(k.label))) return "#d7d9e2";
    return "#3a3e9a";
  }

  // Box of the whole drawing, in layout units.
  let box = $derived({ x: -PAD, y: -TOP - PAD, w: width + 2 * PAD, h: height + TOP + 2 * PAD });
  let screen = $derived({ x: width - SCREEN, y: -TOP - 4 });
  const pct = (v: number, total: number) => `${(v / total) * 100}%`;
</script>

{#if keys.length}
  <div class="art" style:aspect-ratio="{box.w} / {box.h}">
    <svg viewBox="{box.x} {box.y} {box.w} {box.h}" role="img" aria-label="NYXI Striker keyboard">
      <rect x={box.x + 2} y={box.y + 2} width={box.w - 4} height={box.h - 4} rx="14" fill="#4b4fa8" stroke="#2a2c72" stroke-width="3" />
      {#each keys as k (k.slot)}
        <rect x={k.x} y={k.y} width={k.w} height={k.h} rx="4" fill={fill(k)} stroke="rgba(0,0,0,0.35)" stroke-width="1.2" />
      {/each}
      <rect x={screen.x - 3} y={screen.y - 3} width={SCREEN + 6} height={SCREEN + 6} rx="7" fill="#f5c400" />
      <rect x={screen.x} y={screen.y} width={SCREEN} height={SCREEN} rx="5" fill="var(--screen)" />
    </svg>
    {#if shown}
      <div
        class="screen"
        style:left={pct(screen.x + 3 - box.x, box.w)}
        style:top={pct(screen.y + 3 - box.y, box.h)}
        style:width={pct(SCREEN - 6, box.w)}
        style:height={pct(SCREEN - 6, box.h)}
      >
        <DotScreen frames={shown.frames} delay={shown.delay} />
      </div>
    {/if}
  </div>
{/if}

<style>
  .art {
    position: relative;
    width: 100%;
  }
  svg {
    display: block;
    width: 100%;
    height: 100%;
  }
  .screen {
    position: absolute;
  }
</style>
