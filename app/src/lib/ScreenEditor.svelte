<script lang="ts">
  // The editor for one image or animation. Choosing a screen opens it here and shows it on
  // the keyboard; every change is shown on the keyboard as you make it. Built-in screens are
  // read-only: Duplicate makes your own copy. An image is one frame; an animation keeps its
  // frame strip and speed even at one frame. The keyboard plays every frame at one speed.
  import DotScreen from "./DotScreen.svelte";
  import { api, blankFrame, copyFrame, DELAY_UNIT_MS, MAX_FRAMES, type Frame, type Kind, type LibraryItem } from "./api";

  export type Draft = { slug: string | null; user: boolean; kind: Kind; name: string; delay: number; frames: Frame[] };

  let {
    draft,
    onsaved,
    ondeleted,
    oncancel,
    onduplicate,
    onsend,
  }: {
    draft: Draft;
    onsaved: (item: LibraryItem) => void;
    ondeleted: () => void;
    oncancel: (changed: boolean) => void;
    onduplicate: (copy: Draft) => void;
    onsend: (frames: Frame[], delay: number) => void;
  } = $props();

  // The editor works on its own copy; the saved file stays untouched until Save.
  // svelte-ignore state_referenced_locally
  let name = $state(draft.name);
  // svelte-ignore state_referenced_locally
  let delay = $state(draft.delay || 10);
  // svelte-ignore state_referenced_locally
  let frames = $state<Frame[]>(draft.frames.map(copyFrame));
  let current = $state(0);
  let onion = $state(true);
  let dirty = $state(false);
  let confirmCancel = $state(false);
  let confirmDelete = $state(false);
  let saving = $state(false);
  let error = $state("");

  // Fixed for the editor's lifetime (a new editor opens for each screen).
  // svelte-ignore state_referenced_locally
  const readOnly = !draft.user;
  // svelte-ignore state_referenced_locally
  const animation = draft.kind === "animation";
  // svelte-ignore state_referenced_locally
  const saved = draft.slug !== null;
  let fps = $derived((1000 / (Math.max(1, delay) * DELAY_UNIT_MS)).toFixed(1));
  let previous = $derived(animation && frames.length > 1 ? frames[(current - 1 + frames.length) % frames.length] : null);

  const outgoing = () => ({ frames: frames.map(copyFrame), delay: animation ? delay : 0 });

  // ---- painting
  let paintValue: boolean | null = null;

  function paintStart(x: number, y: number, e: PointerEvent) {
    if (readOnly) return;
    e.preventDefault();
    paintValue = !frames[current][y][x];
    paint(x, y);
  }
  function paint(x: number, y: number) {
    if (paintValue === null || frames[current][y][x] === paintValue) return;
    frames[current][y][x] = paintValue;
    changed();
  }

  // Every change goes to the keyboard shortly after it is made (uploads take a moment).
  let timer: ReturnType<typeof setTimeout> | undefined;
  function changed() {
    dirty = true;
    confirmCancel = false;
    confirmDelete = false;
    clearTimeout(timer);
    timer = setTimeout(() => {
      const o = outgoing();
      onsend(o.frames, o.delay);
    }, 300);
  }

  // ---- whole-frame tools
  function transform(f: (src: Frame, x: number, y: number) => boolean) {
    const src = copyFrame(frames[current]);
    frames[current] = src.map((row, y) => row.map((_, x) => f(src, x, y)));
    changed();
  }
  const clear = () => transform(() => false);
  const invert = () => transform((s, x, y) => !s[y][x]);
  const flipH = () => transform((s, x, y) => s[y][8 - x]);
  const flipV = () => transform((s, x, y) => s[8 - y][x]);
  // Shifting wraps around, so nothing is lost and scrolling animations are easy to make.
  const shift = (dx: number, dy: number) => transform((s, x, y) => s[(y - dy + 9) % 9][(x - dx + 9) % 9]);

  // ---- frames
  function addFrame(copy: boolean) {
    if (frames.length >= MAX_FRAMES) return;
    frames.splice(current + 1, 0, copy ? copyFrame(frames[current]) : blankFrame());
    current += 1;
    changed();
  }
  function deleteFrame() {
    if (frames.length < 2) return;
    frames.splice(current, 1);
    current = Math.min(current, frames.length - 1);
    changed();
  }
  function moveFrame(by: number) {
    const to = current + by;
    if (to < 0 || to >= frames.length) return;
    [frames[current], frames[to]] = [frames[to], frames[current]];
    current = to;
    changed();
  }

  // ---- actions
  async function save() {
    saving = true;
    try {
      const o = outgoing();
      clearTimeout(timer);
      const item = await api.saveScreen(draft.slug, name, draft.kind, o.delay, o.frames);
      dirty = false;
      onsaved(item);
    } catch (e) {
      error = String(e);
    } finally {
      saving = false;
    }
  }

  function cancel() {
    if (dirty && !confirmCancel) {
      confirmCancel = true;
      return;
    }
    clearTimeout(timer);
    oncancel(dirty);
  }

  // The copy includes any unsaved changes; this screen itself stays as it was saved.
  function duplicate() {
    clearTimeout(timer);
    const o = outgoing();
    onduplicate({ slug: null, user: true, kind: draft.kind, name: `Copy of ${name.trim() || "Untitled"}`, delay: delay, frames: o.frames });
  }

  async function remove() {
    if (!confirmDelete) {
      confirmDelete = true;
      return;
    }
    try {
      clearTimeout(timer);
      await api.deleteScreen(draft.slug!);
      ondeleted();
    } catch (e) {
      error = String(e);
    }
  }
</script>

<svelte:window onpointerup={() => (paintValue = null)} />

<section class="panel editor">
  <header>
    <div class="title">
      {#if readOnly}
        <h2>{name}</h2>
        <p class="note">Built-in {animation ? "animation" : "image"}. It can't be changed; <b>Duplicate</b> it to make your own copy.</p>
      {:else}
        <span class="label">{saved ? "" : "New "}{animation ? "animation" : "image"}</span>
        <input class="text" type="text" bind:value={name} oninput={() => { dirty = true; confirmCancel = false; }} placeholder="Untitled" />
      {/if}
    </div>
    <div class="actions">
      {#if !readOnly && saved}
        <button class="key" class:danger={confirmDelete} onclick={remove}>{confirmDelete ? "Really delete?" : "Delete"}</button>
      {/if}
      {#if saved}
        <button class="key" onclick={duplicate}>Duplicate</button>
      {/if}
      {#if readOnly}
        <button class="key primary" onclick={cancel}>Done</button>
      {:else}
        <button class="key" class:danger={confirmCancel} onclick={cancel}>{confirmCancel ? "Discard changes?" : "Cancel"}</button>
        <button class="key primary" onclick={save} disabled={saving}>{saving ? "Saving…" : "Save"}</button>
      {/if}
    </div>
  </header>
  {#if error}<p class="error">{error}</p>{/if}

  <div class="body">
    <div>
      <div class="canvas" class:readonly={readOnly}>
        {#each frames[current] as row, y}
          {#each row as on, x}
            <button
              class="cell"
              class:on
              class:ghost={!on && onion && previous?.[y][x]}
              aria-label={`dot ${x + 1},${y + 1}`}
              disabled={readOnly}
              onpointerdown={(e) => paintStart(x, y, e)}
              onpointerenter={() => paint(x, y)}
            ></button>
          {/each}
        {/each}
      </div>
      {#if !readOnly}
        <div class="tools">
          <button class="tool" onclick={clear} title="Clear this frame">Clear</button>
          <button class="tool" onclick={invert} title="Swap lit and dark dots">Invert</button>
          <button class="tool" onclick={flipH} title="Mirror left to right">Flip ↔</button>
          <button class="tool" onclick={flipV} title="Mirror top to bottom">Flip ↕</button>
          <span class="sep"></span>
          <button class="tool" onclick={() => shift(-1, 0)} title="Shift left (wraps)">←</button>
          <button class="tool" onclick={() => shift(1, 0)} title="Shift right (wraps)">→</button>
          <button class="tool" onclick={() => shift(0, -1)} title="Shift up (wraps)">↑</button>
          <button class="tool" onclick={() => shift(0, 1)} title="Shift down (wraps)">↓</button>
        </div>
      {/if}
    </div>

    {#if animation}
      <div class="side">
        <div class="field">
          <span class="label">Preview</span>
          <div class="preview"><DotScreen {frames} {delay} /></div>
        </div>

        <label class="field" class:inactive={readOnly}>
          <span class="label">Speed <b>{fps}</b> frames/s</span>
          <!-- Slider runs fast (left = slow) by inverting the delay. -->
          <input type="range" min="-60" max="-2" value={-delay} disabled={readOnly} oninput={(e) => { delay = -Number(e.currentTarget.value); changed(); }} />
        </label>

        {#if !readOnly}
          <label class="check">
            <input type="checkbox" bind:checked={onion} /> Show previous frame faintly
          </label>
        {/if}
      </div>
    {/if}
  </div>

  {#if animation}
    <div class="frames">
      <div class="frames-head">
        <span class="label">Frames <b>{frames.length}</b> / {MAX_FRAMES}</span>
        {#if !readOnly}
          <div class="frame-tools">
            <button class="tool" onclick={() => addFrame(false)} disabled={frames.length >= MAX_FRAMES}>+ Blank</button>
            <button class="tool" onclick={() => addFrame(true)} disabled={frames.length >= MAX_FRAMES}>+ Copy</button>
            <button class="tool" onclick={() => moveFrame(-1)} disabled={current === 0} title="Move earlier">◀ Move</button>
            <button class="tool" onclick={() => moveFrame(1)} disabled={current === frames.length - 1} title="Move later">Move ▶</button>
            <button class="tool" onclick={deleteFrame} disabled={frames.length < 2}>Delete frame</button>
          </div>
        {/if}
      </div>
      <div class="strip">
        {#each frames as frame, i}
          <button class="thumb" class:active={i === current} onclick={() => (current = i)} title={`Frame ${i + 1}`}>
            <div class="mini"><DotScreen frames={[frame]} playing={false} /></div>
            <span>{i + 1}</span>
          </button>
        {/each}
      </div>
    </div>
  {/if}
</section>

<style>
  header {
    display: flex;
    justify-content: space-between;
    align-items: flex-end;
    gap: 12px;
    margin-bottom: 16px;
  }
  .actions {
    display: flex;
    gap: 8px;
    flex-shrink: 0;
  }
  .title {
    flex: 1;
    min-width: 0;
    max-width: 360px;
  }
  .title h2,
  .title .note {
    margin: 0;
  }
  .readonly .cell {
    cursor: default;
  }
  .readonly .cell:hover {
    outline: none;
  }
  .primary {
    background: var(--accent);
    border-bottom-color: var(--accent-edge);
    color: #1b1b1b;
    font-weight: 600;
  }
  .danger {
    background: #e5484d;
    border-bottom-color: #a8262b;
    color: white;
  }
  .body {
    display: grid;
    grid-template-columns: auto 1fr;
    gap: 28px;
    align-items: start;
  }
  .canvas {
    display: grid;
    grid-template-columns: repeat(9, 38px);
    grid-template-rows: repeat(9, 38px);
    background: var(--screen);
    padding: 10px;
    border-radius: 10px;
    touch-action: none;
    user-select: none;
  }
  .cell {
    margin: 4px;
    border: none;
    border-radius: 50%;
    padding: 0;
    background: rgba(255, 255, 255, 0.09);
    cursor: crosshair;
  }
  .cell:hover {
    outline: 2px solid var(--accent);
  }
  .cell.ghost {
    background: rgba(245, 196, 0, 0.28);
  }
  .cell.on {
    background: #f4f6ff;
    box-shadow: 0 0 5px rgba(255, 255, 255, 0.7);
  }
  .tools,
  .frame-tools {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
    margin-top: 10px;
  }
  .frame-tools {
    margin-top: 0;
  }
  .sep {
    width: 8px;
  }
  .tool {
    background: var(--panel-2);
    color: var(--text);
    border: 1px solid var(--line);
    border-radius: 7px;
    padding: 5px 10px;
    font: inherit;
    font-size: 0.85rem;
    cursor: pointer;
  }
  .tool:hover:not(:disabled) {
    border-color: var(--muted);
  }
  .tool:disabled {
    opacity: 0.4;
    cursor: default;
  }
  .side {
    min-width: 0;
  }
  .text {
    width: 100%;
    box-sizing: border-box;
    background: var(--panel-2);
    color: var(--text);
    border: 1px solid var(--line);
    border-radius: 8px;
    padding: 7px 11px;
    font: inherit;
    font-size: 0.92rem;
  }
  .preview {
    width: 130px;
    height: 130px;
    background: var(--screen);
    border-radius: 10px;
    padding: 8px;
    box-sizing: border-box;
  }
  .check {
    display: flex;
    gap: 8px;
    align-items: center;
    font-size: 0.88rem;
    color: var(--muted);
  }
  .check input {
    accent-color: var(--accent);
  }
  .frames {
    margin-top: 20px;
    border-top: 1px solid var(--line);
    padding-top: 14px;
  }
  .frames-head {
    display: flex;
    justify-content: space-between;
    align-items: center;
    flex-wrap: wrap;
    gap: 8px;
    margin-bottom: 10px;
  }
  .frames-head .label {
    margin: 0;
  }
  .strip {
    display: flex;
    gap: 8px;
    overflow-x: auto;
    padding-bottom: 6px;
  }
  .thumb {
    flex: 0 0 auto;
    background: var(--panel-2);
    border: 2px solid transparent;
    border-radius: 8px;
    padding: 5px 5px 2px;
    color: var(--muted);
    font: inherit;
    font-size: 0.72rem;
    cursor: pointer;
  }
  .thumb.active {
    border-color: var(--accent);
    color: var(--text);
  }
  .mini {
    width: 48px;
    height: 48px;
    background: var(--screen);
    border-radius: 5px;
    padding: 3px;
    box-sizing: border-box;
  }
</style>
