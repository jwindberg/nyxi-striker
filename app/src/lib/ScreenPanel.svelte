<script lang="ts">
  // Images and animations. Choosing one shows it on the keyboard (in its custom slot, which
  // the screen switches to immediately) and opens it in the editor. Your own screens come
  // first in each list; built-in ones are read-only and can be duplicated.
  import { onMount } from "svelte";
  import DotScreen from "./DotScreen.svelte";
  import ScreenEditor, { type Draft } from "./ScreenEditor.svelte";
  import { api, blankFrame, type Frame, type Kind, type LibraryItem } from "./api";

  let { onshow }: { onshow: (frames: Frame[], delay: number) => void } = $props();

  let library = $state<LibraryItem[]>([]);
  let editing = $state<Draft | null>(null);
  // Bumped for every editor opened, so Copy starts a fresh one.
  let editorKey = $state(0);
  // What the keyboard showed before the editor opened; Cancel puts it back.
  let restore: { frames: Frame[]; delay: number } | null = null;
  let lastShown: { frames: Frame[]; delay: number } | null = null;
  let folder = $state("");
  let error = $state("");

  const key = (a: LibraryItem) => (a.user ? "mine:" : "builtin:") + a.slug;
  let images = $derived(library.filter((a) => a.kind === "image"));
  let animations = $derived(library.filter((a) => a.kind === "animation"));

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

  // Uploads are queued so a burst of edits can't interleave on the keyboard; only the
  // latest pending one is sent.
  let sending = false;
  let pending: { frames: Frame[]; delay: number } | null = null;
  async function send(frames: Frame[], delay: number) {
    pending = { frames, delay };
    onshow(frames, delay);
    if (sending) return;
    sending = true;
    while (pending) {
      const next = pending;
      pending = null;
      try {
        await api.play(next.frames, next.delay);
        lastShown = next;
        error = "";
      } catch (e) {
        error = String(e);
      }
    }
    sending = false;
  }

  function open(draft: Draft) {
    editing = draft;
    editorKey += 1;
  }

  function choose(a: LibraryItem) {
    restore = { frames: a.frames, delay: a.delay };
    send(a.frames, a.delay);
    // Blank (first in the collection) has nothing to look at, so it just empties the slot.
    if (!a.user && a.slug === "img-0-blank") return;
    open({ slug: a.slug, user: a.user, kind: a.kind, name: a.name, delay: a.delay, frames: a.frames });
  }

  function create(kind: Kind) {
    restore = lastShown;
    open({ slug: null, user: true, kind, name: "", delay: 10, frames: [blankFrame()] });
  }

  function cancel(changed: boolean) {
    editing = null;
    if (changed && restore) send(restore.frames, restore.delay);
  }

  async function closeAndReload() {
    editing = null;
    await reload();
  }

  function duplicate(copy: Draft) {
    // The copy is shown as it is; Cancel on it goes back to what was showing before.
    restore = lastShown;
    send(copy.frames, copy.kind === "animation" ? copy.delay : 0);
    open(copy);
  }
</script>

{#if editing}
  {#key editorKey}
    <ScreenEditor draft={editing} onsaved={closeAndReload} ondeleted={closeAndReload} oncancel={cancel} onduplicate={duplicate} onsend={send} />
  {/key}
{:else}
  <section class="panel">
    {#if error}<p class="error">{error}</p>{/if}

    {#snippet grid(items: LibraryItem[], kind: Kind)}
      <div class="grid">
        {#each items as a (key(a))}
          <button class="card" onclick={() => choose(a)} title={a.about || a.name}>
            <div class="screen"><DotScreen frames={a.frames} delay={a.delay} /></div>
            <span class="name">{a.name}</span>
            {#if a.user}<span class="mine">yours</span>{/if}
          </button>
        {/each}
        <button class="card new" onclick={() => create(kind)}>
          <div class="screen plus">+</div>
          <span class="name">New {kind}</span>
        </button>
      </div>
    {/snippet}

    <h3>Images</h3>
    {@render grid(images, "image")}
    <h3>Animations</h3>
    {@render grid(animations, "animation")}

    {#if folder}
      <p class="note folder">Your own images and animations are saved on this computer in <code>{folder}</code>. Copy the files there to use them on another computer.</p>
    {/if}
  </section>
{/if}

<style>
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
  h3 {
    margin: 4px 0 10px;
    font-size: 0.78rem;
    text-transform: uppercase;
    letter-spacing: 0.08em;
    color: var(--muted);
    font-weight: 600;
  }
  .grid + h3 {
    margin-top: 22px;
  }
  .grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(104px, 1fr));
    gap: 12px;
  }
  .card {
    position: relative;
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
    border-color: var(--accent);
  }
  .screen {
    width: 100%;
    aspect-ratio: 1;
    background: var(--screen);
    border-radius: 8px;
    padding: 6%;
    box-sizing: border-box;
  }
  .new {
    background: none;
    border: 2px dashed var(--line);
  }
  .plus {
    background: none;
    display: flex;
    align-items: center;
    justify-content: center;
    font-size: 2.2rem;
    color: var(--muted);
  }
  .name {
    font-size: 0.82rem;
    text-align: center;
  }
  .mine {
    position: absolute;
    top: 4px;
    right: 4px;
    background: var(--accent);
    color: #1b1b1b;
    font-size: 0.62rem;
    font-weight: 600;
    padding: 1px 5px;
    border-radius: 5px;
  }
</style>
