<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { open } from '@tauri-apps/plugin-shell';
  import type { Article } from "$lib/types";

  let budget = 100;
  let timeframe = "1w";

  let isLoading = false;
  let articles: Article[] = [];
  let errorMsg: string | null = null;

  async function runAnalysis() {
    isLoading = true;
    errorMsg = null;
    articles = [];
    try {
      const result = await invoke<Article[]>("fetch_top_stories");
      articles = result;
    } catch (e) {
      console.error("Failed to fetch stories:", e);
      errorMsg = e as string;
    } finally {
      isLoading = false;
    }
  }
</script>

<div class="flex h-screen overflow-hidden">
  <!-- SIDEBAR -->
  <aside class="w-64 border-r border-neutral-800 bg-neutral-900/50 p-6 flex flex-col justify-between">
    <div>
      <h1 class="text-2xl font-bold tracking-tight text-emerald-400 mb-8 flex items-center gap-2">
        Kairos
      </h1>

      <nav class="space-y-2">
        <button class="w-full text-left px-4 py-2 rounded-md bg-neutral-800 text-white font-medium shadow-sm ring-1 ring-white/10">
          Market Scanner
        </button>
        <button class="w-full text-left px-4 py-2 rounded-md hover:bg-neutral-800/50 text-neutral-400 transition-colors">
          Portfolio
        </button>
        <button class="w-full text-left px-4 py-2 rounded-md hover:bg-neutral-800/50 text-neutral-400 transition-colors">
          Settings
        </button>
      </nav>
    </div>

    <div class="text-xs text-neutral-600">
      v0.1.0 • Stable
    </div>
  </aside>

  <!-- MAIN CONTENT -->
  <main class="flex-1 overflow-y-auto p-8">
    <header class="mb-10">
      <h2 class="text-3xl font-semibold mb-2">Market Intelligence</h2>
      <p class="text-neutral-400">Configure your parameters to scan prediction markets.</p>
    </header>

    <!-- CONFIGURATION GRID -->
    <div class="grid grid-cols-1 md:grid-cols-3 gap-6 mb-8">
      <!-- Budget Card (no changes) -->
      <div class="p-5 rounded-xl bg-neutral-900 border border-neutral-800 shadow-sm">
        <label for="budget-input" class="block text-sm font-medium text-neutral-400 mb-2">Capital Allocation ($)</label>
        <input id="budget-input" type="number" bind:value={budget} class="w-full bg-neutral-950 border border-neutral-800 rounded-lg px-4 py-3 text-lg focus:outline-none focus:ring-2 focus:ring-emerald-500/50 transition-all"/>
      </div>

      <!-- Timeframe Card (no changes) -->
      <div class="p-5 rounded-xl bg-neutral-900 border border-neutral-800 shadow-sm">
        <label for="timeframe-select" class="block text-sm font-medium text-neutral-400 mb-2">Time Horizon</label>
        <select id="timeframe-select" bind:value={timeframe} class="w-full bg-neutral-950 border border-neutral-800 rounded-lg px-4 py-3 text-lg focus:outline-none focus:ring-2 focus:ring-emerald-500/50 transition-all appearance-none">
          <option value="24h">24 Hours (Day Trade)</option>
          <option value="1w">1 Week (Swing)</option>
          <option value="1m">1 Month (Mid-term)</option>
          <option value="1y">1 Year (Long-term)</option>
        </select>
      </div>

      <!-- Action Card -->
      <div class="flex items-end">
        <!-- This button now calls our Rust function -->
        <button on:click={runAnalysis} disabled={isLoading} class="w-full h-13 bg-emerald-600 hover:bg-emerald-500 text-white font-semibold rounded-xl shadow-lg shadow-emerald-900/20 transition-all active:scale-95 disabled:bg-neutral-600 disabled:cursor-not-allowed">
          {#if isLoading}
            <span>Scanning...</span>
          {:else}
            Run Analysis
          {/if}
        </button>
      </div>
    </div>

    <!-- RESULTS AREA -->
    <div class="border-t border-neutral-800 pt-8">
      {#if isLoading}
        <div class="h-64 border-2 border-dashed border-neutral-800 rounded-xl flex items-center justify-center text-neutral-500">
          Fetching top stories from Hacker News...
        </div>
      {:else if errorMsg}
        <div class="h-64 border-2 border-dashed border-red-800/50 rounded-xl flex items-center justify-center text-red-400 bg-red-900/20 p-4">
          <p><strong>Error:</strong> {errorMsg}</p>
        </div>
      {:else if articles.length > 0}
        <div class="space-y-4">
          {#each articles as article (article.id)}
            <div class="p-4 rounded-lg bg-neutral-900 border border-neutral-800 flex items-center justify-between hover:border-emerald-700 transition-colors">
              <div>
                <button on:click={() => open(article.url)} class="text-left hover:underline">
                  <p class="font-medium">{article.title}</p>
                </button>
                <p class="text-xs text-neutral-400 mt-1">ID: {article.id}</p>
              </div>
              <div class="flex items-center gap-6 text-sm text-center">
                <div>
                  <span class="font-bold">{article.score}</span>
                  <p class="text-xs text-neutral-500">Score</p>
                </div>
                <div>
                  <span class="font-bold">{article.descendants}</span>
                  <p class="text-xs text-neutral-500">Comments</p>
                </div>
              </div>
            </div>
          {/each}
        </div>
      {:else}
        <div class="h-64 border-2 border-dashed border-neutral-800 rounded-xl flex items-center justify-center text-neutral-500">
          Results will appear here...
        </div>
      {/if}
    </div>
  </main>
</div>