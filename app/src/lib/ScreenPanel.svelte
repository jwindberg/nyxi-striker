<script lang="ts">
  // The image and animation library: everything previews live; choosing one uploads it to
  // the keyboard's custom slot, which the screen switches to immediately. An image is
  // simply a one-frame animation. The user's own screens can be created, edited and
  // deleted; built-in ones can be copied and then edited.
  import { onMount } from "svelte";
  import DotScreen from "./DotScreen.svelte";
  import ScreenEditor from "./ScreenEditor.svelte";
  import { api, blankFrame, type Frame, type LibraryItem } from "./api";

  let { onshow }: { onshow: (frames: Frame[], delay: number) => void } = $props();

  type Draft = { slug: string | null; name: string; delay: number; frames: Frame[] };

  let library = $state<LibraryItem[]>([]);
  let selected = $state<LibraryItem | null>(null);
  let editing = $state<Draft | null>(null);
  let folder = $state("");
  let busy = $state(false);
  let confirmDelete = $state(false);
  let error = $state("");

  const key = (a: LibraryItem) => (a.user ? "mine:" : "builtin:") + a.slug;
  let mine = $derived(library.filter((a) => a.user));
  let images = $derived(library.filter((a) => !a.user && a.frames.length === 1));
  let animations = $derived(library.filter((a) => !a.user && a.frames.length > 1));
  let animated = $derived((selected?.frames.length ?? 0) > 1);

  onMount(async () => {
    await reload();
    folder = await api.screensFolder().catch(() => "");
  });

  async function reload() {
    try {
      library = await api.library();
    } catch (e) {
      error = String(e);
    }
  }

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

  function choose(a: LibraryItem) {
    selected = a;
    confirmDelete = false;
    send(a.frames, a.delay);
  }

  function clear() {
    selected = null;
    send([blankFrame()], 0);
  }

  function edit(a: LibraryItem, asCopy: boolean) {
    editing = { slug: asCopy ? null : a.slug, name: asCopy ? `${a.name} 2` : a.name, delay: a.delay, frames: a.frames };
  }

  async function remove() {
    if (!selected?.user) return;
    if (!confirmDelete) {
      confirmDelete = true;
      return;
    }
    try {
      await api.deleteScreen(selected.slug);
      selected = null;
      confirmDelete = false;
      await reload();
    } catch (e) {
      error = String(e);
    }
  }

  async function saved(item: LibraryItem) {
    editing = null;
    await reload();
    choose(library.find((a) => key(a) === key(item)) ?? item);
  }
</script>

{#if editing}
  <ScreenEditor draft={editing} onsaved={saved} onclose={() => (editing = null)} onsend={send} />
{:else}
  <section class="panel">
    <header>
      <div>
        <h2>Screen</h2>
        <p class="note">Choosing an image or animation shows it on the keyboard right away, in its custom slot.</p>
      </div>
      <div class="actions">
        <button class="key" onclick={() => (editing = { slug: null, name: "", delay: 10, frames: [blankFrame()] })}>+ New</button>
        <button class="key" onclick={clear} disabled={busy}>Clear screen</button>
      </div>
    </header>

    {#if error}<p class="error">{error}</p>{/if}

    <!-- Always present so the grid below never shifts. Speed is set in the editor. -->
    <div class="now">
      <span class="label">
        {#if selected}{animated ? "Playing" : "Showing"} <b>{selected.name}</b>{:else}Nothing chosen yet{/if}
      </span>
      <!-- The same three buttons always; built-in screens can't be changed, only duplicated. -->
      <div class="item-actions">
        <button class="tool" disabled={!selected?.user} onclick={() => edit(selected!, false)}>Edit</button>
        <button class="tool" disabled={!selected} onclick={() => edit(selected!, true)}>Duplicate</button>
        <button class="tool" class:danger={confirmDelete} disabled={!selected?.user} onclick={remove}>{confirmDelete ? "Really delete?" : "Delete"}</button>
      </div>
        <p class="hint">
          {#if !selected}Choose a screen below, or press <b>+ New</b> to draw your own.
          {:else if !selected.user}Built-in screens can't be changed. <b>Duplicate</b> one to make your own version.
          {:else}This one is yours: edit it, duplicate it, or delete it.{/if}
        </p>
    </div>

    {#snippet grid(items: LibraryItem[])}
      <div class="grid">
        {#each items as a (key(a))}
          <button class="card" class:active={selected && key(selected) === key(a)} onclick={() => choose(a)} title={a.about || a.name} disabled={busy}>
            <div class="screen"><DotScreen frames={a.frames} delay={a.delay} /></div>
            <span class="name">{a.name}</span>
          </button>
        {/each}
      </div>
    {/snippet}

    {#if mine.length}
      <h3>Mine</h3>
      {@render grid(mine)}
    {/if}
    <h3>Images</h3>
    {@render grid(images)}
    <h3>Animations</h3>
    {@render grid(animations)}

    {#if folder}
      <p class="note folder">Your own images and animations are saved on this computer in <code>{folder}</code>. Copy the files there to use them on another computer.</p>
    {/if}
  </section>
{/if}

<style>
  header {
    display: flex;
    gap: 16px;
    justify-content: space-between;
    align-items: flex-start;
    margin-bottom: 14px;
  }
  .actions,
  .item-actions {
    display: flex;
    gap: 6px;
  }
  .hint {
    margin: 6px 0 8px;
    font-size: 0.8rem;
    color: var(--muted);
  }
  .tool {
    background: var(--panel);
    color: var(--text);
    border: 1px solid var(--line);
    border-radius: 7px;
    padding: 4px 10px;
    font: inherit;
    font-size: 0.82rem;
    cursor: pointer;
  }
  .tool:disabled {
    opacity: 0.4;
  }
  .danger {
    background: #e5484d;
    border-color: #a8262b;
    color: white;
  }
  .folder {
    margin-top: 18px;
  }
  code {
    font-size: 0.8rem;
    background: var(--panel-2);
    padding: 1px 5px;
    border-radius: 4px;
    user-select: all;
    word-break: break-all;
  }
  .now {
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
