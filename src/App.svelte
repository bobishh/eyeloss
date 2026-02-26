<script>
  import { onMount } from 'svelte';
  import { fade } from 'svelte/transition';
  import Sloposcope from './lib/Sloposcope.svelte';
  import { getFortune } from './lib/fortunes.js';
  const CHANGES_PAGE_SIZE = 20;
  const ALL_CHANGES_LIMIT = 50000;
  const DEBUG_LOGS = (() => {
    try {
      if (typeof window === 'undefined') return false;
      const qs = new URLSearchParams(window.location.search);
      if (qs.get('debug') === '1') return true;
      if (qs.get('debug') === '0') return false;
      const stored = localStorage.getItem('sloposcope-debug') ?? localStorage.getItem('eyeloss-debug');
      if (stored === '1') return true;
      if (stored === '0') return false;
      return import.meta.env.DEV;
    } catch (_e) {
      return import.meta.env.DEV;
    }
  })();

  function debugLog(...args) {
    if (DEBUG_LOGS) console.log(...args);
  }

  const MOCK_CHANGES = [
    { id: 'deadbeef', description: 'Working copy', timestamp: '2026-02-24 23:00' },
    { id: 'c0ffee11', description: 'Refactor parser pipeline', timestamp: '2026-02-24 22:30' },
    { id: 'b0ba0000', description: 'Fix timeline hover metadata', timestamp: '2026-02-24 22:00' },
    { id: 'fadedcab', description: 'Improve graph edge resolver', timestamp: '2026-02-24 21:30' },
    { id: 'decafbad', description: 'Add file watcher heat map', timestamp: '2026-02-24 21:00' },
    { id: 'bead1234', description: 'Tidy css variables', timestamp: '2026-02-24 20:30' },
    { id: '8badf00d', description: 'Add bookmarks API', timestamp: '2026-02-24 20:00' },
    { id: 'facefeed', description: 'Stabilize canvas zoom', timestamp: '2026-02-24 19:30' },
    { id: 'ab12cd34', description: 'Introduce code window', timestamp: '2026-02-24 19:00' },
    { id: '3412dcba', description: 'Bootstrap initial graph renderer', timestamp: '2026-02-24 18:30' },
  ];

  function uniqueChanges(list) {
    const seen = new Set();
    const result = [];
    for (const change of list || []) {
      const id = change?.id;
      if (!id || seen.has(id)) continue;
      seen.add(id);
      result.push(change);
    }
    return result;
  }

  // Tauri API mocks for browser/test environment
  let invoke = async (cmd, args) => {
    if (window.__TAURI_INTERNALS__) {
      const { invoke: tauriInvoke } = await import('@tauri-apps/api/core');
      debugLog(`[FRONTEND] Invoking ${cmd}`, args);
      return tauriInvoke(cmd, args);
    }
    debugLog('[MOCK] invoke', cmd, args);
    if (cmd === 'get_graph') return { nodes: [{ id: 'MockModule', type: 'module', file: 'mock.ex', line_count: 42, change_status: 'unchanged', functions: [] }], edges: [] };
    if (cmd === 'get_changes') {
      const limit = Math.max(1, Number(args?.limit || 20));
      const beforeId = args?.before_id;
      if (!beforeId) return MOCK_CHANGES.slice(0, limit);
      const cursorIndex = MOCK_CHANGES.findIndex(c => c.id === beforeId);
      if (cursorIndex < 0) return [];
      return MOCK_CHANGES.slice(cursorIndex + 1, cursorIndex + 1 + limit);
    }
    if (cmd === 'get_bookmarks') return [{ name: 'main', id: 'c0ffee11' }];
    if (cmd === 'get_current_branch') return 'main';
    if (cmd === 'get_repo_path') return '/path/to/despair';
    if (cmd === 'select_repo') return '/new/path/to/despair';
    if (cmd === 'get_file_source') return 'defmodule MockModule do\n  def hello, do: :world\nend';
    if (cmd === 'save_file') { debugLog('[MOCK] saving file', args); return null; }
    return null;
  };

  let listen = async (event, cb) => {
    if (window.__TAURI_INTERNALS__) {
      const { listen: tauriListen } = await import('@tauri-apps/api/event');
      return tauriListen(event, cb);
    }
    debugLog('[MOCK] listen', event);
    return () => {};
  };

  let graph = $state({ nodes: [], edges: [] });
  let changes = $state([]);
  let changesLimit = $state(CHANGES_PAGE_SIZE);
  let hasMoreChanges = $state(true);
  let bookmarks = $state([]);
  let currentBranch = $state('');
  let since = $state('@'); 
  let loading = $state(true); // Initial load
  let refreshing = $state(false); // Update state
  let currentFortune = $state('');
  let repoPath = $state('');
  let showDropdown = $state(false);
  let highlightedIndex = $state(-1);

  // Heatmap state
  let heatmapData = $state(new Map());
  let heatCounter = 0;

  function refreshFortune() {
    try {
      updateFortune();
    } catch (_e) {
      // no-op
    }
    if (!currentFortune || currentFortune.trim() === '') {
      currentFortune = getFortune([]);
    }
  }

  function updateFortune() {
    const committers = changes.map(c => c.description.split(' ')[0]).filter(n => n && n.length > 2);
    currentFortune = getFortune(committers);
  }

  function getShortName(id) {
    if (!id) return '';
    
    if (id.includes('/')) {
      const parts = id.split('/');
      return parts[parts.length - 1]; // Return full filename: "config.exs"
    }
    
    if (id.includes('.')) {
      const parts = id.split('.');
      const last = parts[parts.length - 1];
      
      // Known file extensions
      const knownExts = ['ex', 'exs', 'svelte', 'js', 'ts', 'jsx', 'tsx', 'rs', 'py', 'rb', 'go', 'java', 'cpp', 'h', 'php', 'cs', 'json', 'md'];
      if (knownExts.includes(last.toLowerCase())) {
        return id; 
      }
      return last;
    }
    
    return id;
  }

  let filteredItems = $derived.by(() => {
    const q = (since || '').toLowerCase();
    
    const matchedBookmarks = bookmarks
      .filter(b => b.name.toLowerCase().includes(q))
      .map(b => ({ id: b.name, description: 'Bookmark', type: 'bookmark' }));
      
    const matchedChanges = changes
      .filter(c => c.id.toLowerCase().includes(q) || c.description.toLowerCase().includes(q))
      .map(c => ({ ...c, type: 'commit' }));

    return [...matchedBookmarks, ...matchedChanges];
  });

  async function loadTimelineChanges() {
    const c = await invoke('get_changes', { limit: ALL_CHANGES_LIMIT });
    const unique = uniqueChanges(c);
    changes = unique;
    changesLimit = unique.length;
    hasMoreChanges = false; // We paginate locally after prefetching full history.
  }

  async function loadGraph(isInitial = false) {
    refreshFortune();

    if (isInitial) loading = true;
    else refreshing = true;

    // Safety timeout to clear loader if backend hangs
    const safetyTimeout = setTimeout(() => {
      loading = false;
      refreshing = false;
    }, 10000);

    const sinceArg = since && since !== '' ? since : null;
    try {
      debugLog(`[FRONTEND] Starting loadGraph (initial: ${isInitial}) since: ${sinceArg}`);
      const [g, b, curr] = await Promise.all([
        invoke('get_graph', { since: sinceArg }),
        invoke('get_bookmarks'),
        invoke('get_current_branch'),
      ]);

      debugLog(`[FRONTEND] Received data. Nodes: ${g.nodes.length}`);
      graph = g;
      bookmarks = b;
      currentBranch = curr;
    } catch (e) {
      console.error('[FRONTEND] Error loading graph:', e);
    } finally {
      clearTimeout(safetyTimeout);
      loading = false;
      refreshing = false;
    }
  }

  let loadingMore = $state(false);
  async function loadMoreChanges() {
    if (loadingMore || changes.length === 0 || !hasMoreChanges) return 0;
    refreshFortune();
    loadingMore = true;
    let loadedCount = 0;
    try {
      const targetLimit = Math.max(changes.length, changesLimit) + CHANGES_PAGE_SIZE;
      const expanded = await invoke('get_changes', { limit: targetLimit });
      const expandedUnique = uniqueChanges(expanded);
      const existingIds = new Set(changes.map(c => c.id));
      const tail = expandedUnique.filter(c => !existingIds.has(c.id));

      if (tail.length > 0) {
        changes = [...changes, ...tail];
        loadedCount = tail.length;
      }

      changesLimit = Math.max(changesLimit, expandedUnique.length);
      if (expandedUnique.length < targetLimit || loadedCount === 0) {
        hasMoreChanges = false;
      }
    } catch (e) {
      console.error('[FRONTEND] Error loading more changes:', e);
    } finally {
      loadingMore = false;
    }
    return loadedCount;
  }

  function setSince(revset, event) {
    if (event && (event.shiftKey || event.metaKey || event.ctrlKey)) {
      // Toggle logic for multi-select
      const current = (since || '').split(' | ').filter(s => s && s !== '');
      if (current.includes(revset)) {
        since = current.filter(s => s !== revset).join(' | ');
      } else {
        since = [...current, revset].join(' | ');
      }
    } else {
      since = revset;
    }
    
    if (!since || since === '') since = '@';
    debugLog(`[FRONTEND] Setting since to: ${since}`);
    loadGraph(false);
  }

  async function selectRepo() {
    try {
      const newPath = await invoke('select_repo');
      repoPath = newPath;
      since = '@';
      await Promise.all([loadTimelineChanges(), loadGraph(true)]);
    } catch (e) {
      debugLog('Dialog cancelled or failed', e);
    }
  }

  let theme = $state(typeof localStorage !== 'undefined' ? (localStorage.getItem('codegraph-theme') || 'midnight') : 'midnight');

  function toggleTheme() {
    theme = theme === 'midnight' ? 'victorian' : 'midnight';
    if (typeof localStorage !== 'undefined') localStorage.setItem('codegraph-theme', theme);
    applyTheme();
  }

  function applyTheme() {
    document.documentElement.setAttribute('data-theme', theme);
  }

  async function getFileSource(file) {
    return invoke('get_file_source', { file });
  }

  async function getFileDiff(file) {
    return invoke('get_file_diff', { file, since: since === '@' ? null : since });
  }

  async function saveFile(file, content) {
    return invoke('save_file', { file, content });
  }

  onMount(async () => {
    applyTheme();
    refreshFortune();
    repoPath = await invoke('get_repo_path');
    await Promise.all([loadTimelineChanges(), loadGraph(true)]);

    listen('graph-updated', (event) => {
      debugLog('[FRONTEND] Graph updated event received');
      graph = event.payload.graph;
      if (event.payload.current_branch) {
        currentBranch = event.payload.current_branch;
      }
      if (Array.isArray(event.payload.bookmarks)) {
        bookmarks = event.payload.bookmarks;
      }
      loadTimelineChanges().catch((e) => console.error('[FRONTEND] Failed to refresh timeline changes:', e));
    });

    listen('file-touched', (event) => {
      const path = event.payload;
      debugLog(`[FRONTEND] File touched: ${path}`);
      
      heatmapData.set(path, { seq: ++heatCounter, touchedAt: Date.now() });
      if (heatmapData.size > 100) {
        let oldestKey = null;
        let oldestSeq = Infinity;
        for (const [key, value] of heatmapData.entries()) {
          const seq = typeof value === 'number' ? value : Number(value?.seq ?? 0);
          if (seq < oldestSeq) {
            oldestSeq = seq;
            oldestKey = key;
          }
        }
        if (oldestKey) heatmapData.delete(oldestKey);
      }
      // Force reactivity in Svelte 5 for Map
      heatmapData = new Map(heatmapData);
    });
  });

</script>

<div class="eyeloss-page">
  <div class="eyeloss-controls">
    <div class="eyeloss-controls__left">
      <div class="eyeloss-controls__row">
        <span class="eyeloss-controls__repo">{repoPath}</span>
        <button type="button" class="btn btn-ghost" onclick={selectRepo} style="font-size: 0.6rem; padding: 2px 6px;">Change</button>
      </div>
      <div class="eyeloss-controls__row">
        <span class="eyeloss-controls__branch">branch: {currentBranch}</span>
      </div>
    </div>

    <div class="eyeloss-controls__center">
      <form 
        onsubmit={(e) => { e.preventDefault(); setSince(since); showDropdown = false; }}
        style="position: relative;"
      >
        <label for="since-input">Since</label>
        <div style="position: relative; display: inline-block;">
          <input
            id="since-input"
            name="since"
            type="text"
            bind:value={since}
            onfocus={() => showDropdown = true}
            onblur={() => setTimeout(() => showDropdown = false, 200)}
            onkeydown={(e) => {
              if (e.key === 'ArrowDown') {
                e.preventDefault();
                highlightedIndex = Math.min(highlightedIndex + 1, filteredItems.length - 1);
              } else if (e.key === 'ArrowUp') {
                e.preventDefault();
                highlightedIndex = Math.max(highlightedIndex - 1, 0);
              } else if (e.key === 'Enter' && highlightedIndex >= 0) {
                e.preventDefault();
                setSince(filteredItems[highlightedIndex].id);
                showDropdown = false;
              } else if (e.key === 'Escape') {
                showDropdown = false;
              }
            }}
            placeholder="@ | @- | ancestors(@, 5)"
            class="input-mono"
            autocomplete="off"
          />
          {#if showDropdown && filteredItems.length > 0}
            <div class="eyeloss-dropdown">
              {#each filteredItems as item, i}
                <button
                  type="button"
                  class="eyeloss-dropdown__item"
                  class:eyeloss-dropdown__item--highlighted={i === highlightedIndex}
                  onclick={() => { setSince(item.id); showDropdown = false; }}
                >
                  <span class="eyeloss-dropdown__id">
                    {getShortName(item.id)} 
                    {#if item.type === 'bookmark'}<small style="opacity: 0.5; font-weight: normal; margin-left: 4px;">(bookmark)</small>{/if}
                  </span>
                  <span class="eyeloss-dropdown__desc">{item.description}</span>
                </button>
              {/each}
            </div>
          {/if}
        </div>
        <button type="submit" class="btn btn-primary">Apply</button>
        {#if since}
          <button type="button" class="btn btn-ghost" onclick={() => { setSince(null); since = ''; }}>Clear</button>
        {/if}
      </form>
    </div>

    <div class="eyeloss-controls__right">
      <button type="button" class="btn btn-ghost theme-toggle" onclick={toggleTheme} title="Toggle Theme">
        {theme === 'midnight' ? '☀️' : '🌙'}
      </button>
    </div>
  </div>

  <Sloposcope {graph} 
    {since} 
    {changes} 
    {bookmarks} 
    {getFileDiff} 
    {getFileSource} 
    {saveFile} 
    {theme} 
    {heatmapData}
    {loadingMore}
    {hasMoreChanges}
    onSelectSince={setSince} 
    onLoadMoreChanges={loadMoreChanges}
  />

  {#if loading || refreshing || loadingMore}
    <div class={loading ? 'eyeloss-splash' : 'eyeloss-refresh-overlay'} transition:fade={{ duration: 120 }}>
      <div class="eyeloss-splash__fortune">{currentFortune}</div>
    </div>
  {/if}
</div>
