<script lang="ts">
  // Keyboard settings. Each control writes to the keyboard as soon as it changes.
  import { onMount } from "svelte";
  import { api, type Info, type Settings, type SleepTimers } from "./api";
  import { getVersion } from "@tauri-apps/api/app";

  let { info }: { info: Info | null } = $props();

  let settings = $state<Settings | null>(null);
  let error = $state("");

  const RATES = [1000, 500, 250, 125];
  // The official app's limits, in minutes.
  const LIGHT_MIN = 1;
  const DEEP_MIN = 10;
  const MAX_MIN = 60;

  let appVersion = $state("");

  onMount(() => {
    load();
    getVersion().then((v) => (appVersion = v), () => {});
  });

  async function load() {
    try {
      settings = await api.getSettings();
      error = "";
    } catch (e) {
      error = String(e);
    }
  }

  async function run(write: Promise<void>) {
    try {
      await write;
      error = "";
    } catch (e) {
      error = String(e);
      await load();
    }
  }

  let debounceTimer: ReturnType<typeof setTimeout> | undefined;
  function changeDebounce() {
    clearTimeout(debounceTimer);
    debounceTimer = setTimeout(() => run(api.setDebounce(settings!.debounce_ms)), 300);
  }

  function setSleep(field: keyof SleepTimers, minutes: number, min: number) {
    if (!settings || !Number.isFinite(minutes)) return;
    settings.sleep[field] = Math.round(Math.min(MAX_MIN, Math.max(min, minutes)) * 60);
    run(api.setSleepTimers(settings.sleep));
  }

  const version = (v: number) => `${v >> 8}.${String(v & 0xff).padStart(2, "0")}`;
</script>

<div class="stack">
  <section class="panel">
    <h2>Keyboard</h2>
    {#if error}<p class="error">{error} <button class="link" onclick={load}>Retry</button></p>{/if}

    {#if settings}
      <div class="grid">
        <label class="field">
          <span class="label">Polling rate</span>
          <select value={settings.polling_hz} onchange={(e) => { settings!.polling_hz = Number(e.currentTarget.value); run(api.setPollingRate(settings!.polling_hz)); }}>
            {#each RATES as hz}
              <option value={hz}>{hz} Hz</option>
            {/each}
          </select>
        </label>

        <label class="field">
          <span class="label">Debounce <b>{settings.debounce_ms} ms</b></span>
          <input type="range" min="0" max="10" bind:value={settings.debounce_ms} oninput={changeDebounce} />
        </label>

        <div class="field">
          <span class="label">Windows key</span>
          <button class="toggle" class:on={settings.win_lock} role="switch" aria-checked={settings.win_lock}
            onclick={() => { settings!.win_lock = !settings!.win_lock; run(api.setWinLock(settings!.win_lock)); }}>
            <span class="knob"></span>
            {settings.win_lock ? "Locked" : "Unlocked"}
          </button>
        </div>

        <label class="field">
          <span class="label">Fn shortcuts for</span>
          <select value={settings.mac_mode ? "mac" : "win"} onchange={(e) => { settings!.mac_mode = e.currentTarget.value === "mac"; run(api.setMacMode(settings!.mac_mode)); }}>
            <option value="win">Windows</option>
            <option value="mac">macOS</option>
          </select>
          <span class="hint">macOS changes Fn + F3, F4 and F12 to Mission Control, minimize and input switching, and turns off Fn + Win.</span>
        </label>
      </div>
    {:else if !error}
      <p class="note">Reading the keyboard…</p>
    {/if}
  </section>

  {#if settings}
    <section class="panel">
      <h2>Sleep</h2>
      <p class="note">Only used on 2.4 GHz and Bluetooth; a wired keyboard never sleeps. Light sleep dims the keyboard, deep sleep turns it off until a key is pressed.</p>
      <table>
        <thead>
          <tr><th></th><th>2.4 GHz</th><th>Bluetooth</th></tr>
        </thead>
        <tbody>
          {#each [["Light sleep after", "light_24g", "light_bt", LIGHT_MIN], ["Deep sleep after", "deep_24g", "deep_bt", DEEP_MIN]] as const as [title, f24, fbt, min]}
            <tr>
              <th>{title}</th>
              {#each [f24, fbt] as field}
                <td>
                  <input type="number" min={min} max={MAX_MIN} value={Math.round(settings.sleep[field] / 60)}
                    onchange={(e) => setSleep(field, Number(e.currentTarget.value), min)} />
                  <span class="unit">min</span>
                </td>
              {/each}
            </tr>
          {/each}
        </tbody>
      </table>
    </section>
  {/if}

  <section class="panel">
    <h2>About</h2>
    <dl>
      <dt>App</dt><dd>NYXI Striker {appVersion}</dd>
      {#if info}
        <dt>Keyboard</dt><dd>NYXI Striker, model CA01 (device {info.device_id})</dd>
        <dt>Connection</dt><dd>{info.wireless ? "2.4 GHz receiver" : "USB cable"}</dd>
        <dt>Firmware</dt><dd>{version(info.firmware)}</dd>
      {:else}
        <dt>Keyboard</dt><dd>Not connected</dd>
      {/if}
    </dl>
  </section>
</div>

<style>
  .stack {
    display: grid;
    gap: 14px;
  }
  h2 {
    margin-bottom: 14px !important;
  }
  .grid {
    display: grid;
    grid-template-columns: repeat(3, 1fr);
    gap: 0 24px;
    align-items: start;
  }
  .hint {
    display: block;
    margin-top: 6px;
    color: var(--muted);
    font-size: 0.8rem;
    line-height: 1.4;
  }
  .toggle {
    display: inline-flex;
    align-items: center;
    gap: 10px;
    background: none;
    border: none;
    color: var(--text);
    font: inherit;
    font-size: 0.9rem;
    cursor: pointer;
    padding: 4px 0;
  }
  .knob {
    width: 38px;
    height: 22px;
    border-radius: 11px;
    background: var(--panel-2);
    border: 1px solid var(--line);
    position: relative;
    transition: background 0.15s;
  }
  .knob::after {
    content: "";
    position: absolute;
    top: 2px;
    left: 2px;
    width: 16px;
    height: 16px;
    border-radius: 50%;
    background: var(--key);
    transition: left 0.15s;
  }
  .toggle.on .knob {
    background: var(--accent);
    border-color: var(--accent-edge);
  }
  .toggle.on .knob::after {
    left: 18px;
  }
  table {
    border-collapse: collapse;
    margin-top: 12px;
  }
  th,
  td {
    text-align: left;
    padding: 5px 18px 5px 0;
    font-size: 0.9rem;
  }
  thead th {
    font-size: 0.78rem;
    text-transform: uppercase;
    letter-spacing: 0.08em;
    color: var(--muted);
    font-weight: normal;
  }
  tbody th {
    font-weight: normal;
    color: var(--muted);
  }
  input[type="number"] {
    width: 60px;
    background: var(--panel-2);
    color: var(--text);
    border: 1px solid var(--line);
    border-radius: 7px;
    padding: 5px 8px;
    font: inherit;
  }
  .unit {
    color: var(--muted);
    font-size: 0.85rem;
    margin-left: 4px;
  }
  dl {
    display: grid;
    grid-template-columns: max-content 1fr;
    gap: 6px 18px;
    margin: 10px 0 0;
    font-size: 0.9rem;
  }
  dt {
    color: var(--muted);
  }
  dd {
    margin: 0;
  }
</style>
