<script lang="ts">
  import { browser } from "$app/environment";
  import { onDestroy, onMount } from "svelte";
  import type { Recommendation } from "$lib/types";

  let budget = 100;

  let isLoading = false;
  let recommendations: Recommendation[] = [];
  let errorMsg: string | null = null;
  let loadingStatus = "";
  let progress = 0;
  let statusSteps: string[] = [];
  $: lowRisk = recommendations.filter((r) => r.risk_level === "low-risk");
  $: medRisk = recommendations.filter((r) => r.risk_level === "medium-risk");
  $: highRisk = recommendations.filter((r) => r.risk_level === "high-risk");

  let unlisten: null | (() => void) = null;
  let unlistenResult: null | (() => void) = null;
  let unlistenError: null | (() => void) = null;
  onMount(async () => {
    if (!browser) return;
    const { listen } = await import("@tauri-apps/api/event");
    unlisten = await listen<{ step: string; message: string; progress: number }>(
      "analysis_status",
      (event) => {
        const payload = event.payload;
        if (!payload) return;
        loadingStatus = payload.message;
        progress = Math.max(0, Math.min(100, Math.round(payload.progress * 100)));
        statusSteps = [...statusSteps, payload.message].slice(-6);
      }
    );
    unlistenResult = await listen<Recommendation[]>("analysis_result", (event) => {
      recommendations = event.payload ?? [];
      isLoading = false;
    });
    unlistenError = await listen<string>("analysis_error", (event) => {
      errorMsg = event.payload ?? "Analysis failed.";
      isLoading = false;
    });
  });

  onDestroy(() => {
    if (unlisten) {
      unlisten();
      unlisten = null;
    }
    if (unlistenResult) {
      unlistenResult();
      unlistenResult = null;
    }
    if (unlistenError) {
      unlistenError();
      unlistenError = null;
    }
  });

  async function openExternal(url: string) {
    if (!browser) return;
    const { open } = await import("@tauri-apps/plugin-opener");
    await open(url);
  }

  async function runAnalysis() {
    isLoading = true;
    errorMsg = null;
    recommendations = [];
    loadingStatus = "Starting analysis...";
    progress = 0;
    statusSteps = [];

    try {
      if (!browser) {
        throw new Error("Tauri API not available in this context.");
      }
      const { invoke } = await import("@tauri-apps/api/core");
      await invoke("run_full_analysis", { budget });
    } catch (e) {
      console.error("Analysis failed:", e);
      errorMsg = e as string;
      isLoading = false;
    } finally {
      // isLoading will be cleared when analysis_result or analysis_error arrives.
    }
  }
</script>

<div class="flex h-screen overflow-hidden">
  <aside class="w-64 border-r border-neutral-800 bg-neutral-900/50 p-6 flex flex-col justify-between">
    <div>
      <h1 class="text-2xl font-bold tracking-tight text-emerald-400 mb-8 flex items-center gap-2">
        Kairos
      </h1>

      <div class="px-3 py-2 rounded-md bg-neutral-800 text-white font-medium shadow-sm ring-1 ring-white/10">
        Market Scanner
      </div>
      <p class="mt-4 text-xs text-neutral-500 leading-relaxed">
        Signals are informational only. Always verify sources before allocating capital.
      </p>
    </div>

    <div class="text-xs text-neutral-600">
      v0.1.0 • Stable
    </div>
  </aside>

  <main class="flex-1 overflow-y-auto p-8">
    <header class="mb-10">
      <h2 class="text-3xl font-semibold mb-2">Market Intelligence</h2>
      <p class="text-neutral-400">Configure your parameters to scan prediction markets.</p>
    </header>

    <div class="grid grid-cols-1 md:grid-cols-2 gap-6 mb-8">
      <div class="p-5 rounded-xl bg-neutral-900 border border-neutral-800 shadow-sm">
        <label for="budget-input" class="block text-sm font-medium text-neutral-400 mb-2">Capital Allocation ($)</label>
        <input id="budget-input" type="number" bind:value={budget} class="w-full bg-neutral-950 border border-neutral-800 rounded-lg px-4 py-3 text-lg focus:outline-none focus:ring-2 focus:ring-emerald-500/50 transition-all"/>
      </div>

      <div class="flex items-end">
        <button on:click={runAnalysis} disabled={isLoading} class="w-full h-13 bg-emerald-600 hover:bg-emerald-500 text-white font-semibold rounded-xl shadow-lg shadow-emerald-900/20 transition-all active:scale-95 disabled:bg-neutral-600 disabled:cursor-not-allowed">
          {#if isLoading}
            <span>Scanning...</span>
          {:else}
            Run Analysis
          {/if}
        </button>
      </div>
    </div>

    <div class="border-t border-neutral-800 pt-8">
      {#if isLoading}
        <div class="h-64 border-2 border-dashed border-neutral-800 rounded-xl flex flex-col items-center justify-center text-neutral-400 px-6">
          <p class="text-lg font-medium">{loadingStatus}</p>
          <p class="text-sm text-neutral-500 mt-2">This may take a minute...</p>
          <div class="w-full max-w-xl mt-6">
            <div class="h-2 rounded-full bg-neutral-800 overflow-hidden">
              <div
                class="h-full bg-emerald-500 transition-all duration-500"
                style={`width: ${progress}%`}
              />
            </div>
            <div class="mt-3 text-xs text-neutral-500">
              {progress}% complete
            </div>
            {#if statusSteps.length > 0}
              <div class="mt-3 text-xs text-neutral-600 space-y-1">
                {#each statusSteps as step}
                  <div>• {step}</div>
                {/each}
              </div>
            {/if}
          </div>
        </div>
      {:else if errorMsg}
        <div class="h-64 border-2 border-dashed border-red-800/50 rounded-xl flex items-center justify-center text-red-400 bg-red-900/20 p-4">
          <p><strong>Error:</strong> {errorMsg}</p>
        </div>
      {:else if recommendations.length > 0}
        <div class="space-y-8">
          <div class="flex flex-wrap items-center gap-3 text-xs text-neutral-500">
            <span class="px-2 py-1 rounded-full bg-neutral-800 text-neutral-300">Low risk: {lowRisk.length}</span>
            <span class="px-2 py-1 rounded-full bg-neutral-800 text-neutral-300">Medium risk: {medRisk.length}</span>
            <span class="px-2 py-1 rounded-full bg-neutral-800 text-neutral-300">High risk: {highRisk.length}</span>
          </div>

          {#if lowRisk.length > 0}
            <section class="space-y-3">
              <h3 class="text-sm uppercase tracking-wider text-emerald-400">Low Risk</h3>
              <div class="grid grid-cols-1 md:grid-cols-2 xl:grid-cols-3 gap-4">
                {#each lowRisk as rec (rec.market_slug)}
                  <div class="p-4 rounded-lg bg-neutral-900 border border-emerald-900/40 hover:border-emerald-700 transition-colors">
                    <div class="flex items-start justify-between gap-4">
                      <div>
                        <p class="font-semibold text-base leading-snug">{rec.market_question}</p>
                        <button on:click={() => openExternal(`https://polymarket.com/event/${rec.market_slug}`)} class="text-xs text-neutral-500 hover:underline">
                          View on Polymarket ↗
                        </button>
                      </div>
                      <div class="text-right shrink-0">
                        <p class="text-xs text-neutral-400">Bet</p>
                        <p class="text-xl font-bold {rec.suggested_outcome === 'YES' ? 'text-green-400' : 'text-red-400'}">
                          {rec.suggested_outcome}
                        </p>
                        <p class="text-xs font-mono text-neutral-400">{rec.market_price.toFixed(2)}¢</p>
                      </div>
                    </div>
                    <div class="mt-3 flex items-center justify-between text-xs text-neutral-400">
                      <span>Stake ${rec.suggested_budget.toFixed(2)} on {rec.suggested_outcome}</span>
                      <span class="px-2 py-0.5 rounded-full bg-emerald-900/30 text-emerald-200">{rec.risk_level}</span>
                    </div>
                    <div class="mt-3 pt-3 border-t border-neutral-800">
                      <p class="text-xs text-neutral-400">{rec.reasoning}</p>
                      <p class="text-[11px] text-neutral-600 mt-2">
                        {rec.source_article_title}
                        {#if rec.source}
                          <span class="ml-1">({rec.source})</span>
                        {/if}
                      </p>
                    </div>
                  </div>
                {/each}
              </div>
            </section>
          {/if}

          {#if medRisk.length > 0}
            <section class="space-y-3">
              <h3 class="text-sm uppercase tracking-wider text-amber-300">Medium Risk</h3>
              <div class="grid grid-cols-1 md:grid-cols-2 xl:grid-cols-3 gap-4">
                {#each medRisk as rec (rec.market_slug)}
                  <div class="p-4 rounded-lg bg-neutral-900 border border-amber-900/40 hover:border-amber-700 transition-colors">
                    <div class="flex items-start justify-between gap-4">
                      <div>
                        <p class="font-semibold text-base leading-snug">{rec.market_question}</p>
                        <button on:click={() => openExternal(`https://polymarket.com/event/${rec.market_slug}`)} class="text-xs text-neutral-500 hover:underline">
                          View on Polymarket ↗
                        </button>
                      </div>
                      <div class="text-right shrink-0">
                        <p class="text-xs text-neutral-400">Bet</p>
                        <p class="text-xl font-bold {rec.suggested_outcome === 'YES' ? 'text-green-400' : 'text-red-400'}">
                          {rec.suggested_outcome}
                        </p>
                        <p class="text-xs font-mono text-neutral-400">{rec.market_price.toFixed(2)}¢</p>
                      </div>
                    </div>
                    <div class="mt-3 flex items-center justify-between text-xs text-neutral-400">
                      <span>Stake ${rec.suggested_budget.toFixed(2)} on {rec.suggested_outcome}</span>
                      <span class="px-2 py-0.5 rounded-full bg-amber-900/30 text-amber-200">{rec.risk_level}</span>
                    </div>
                    <div class="mt-3 pt-3 border-t border-neutral-800">
                      <p class="text-xs text-neutral-400">{rec.reasoning}</p>
                      <p class="text-[11px] text-neutral-600 mt-2">
                        {rec.source_article_title}
                        {#if rec.source}
                          <span class="ml-1">({rec.source})</span>
                        {/if}
                      </p>
                    </div>
                  </div>
                {/each}
              </div>
            </section>
          {/if}

          {#if highRisk.length > 0}
            <section class="space-y-3">
              <h3 class="text-sm uppercase tracking-wider text-red-300">High Risk</h3>
              <div class="grid grid-cols-1 md:grid-cols-2 xl:grid-cols-3 gap-4">
                {#each highRisk as rec (rec.market_slug)}
                  <div class="p-4 rounded-lg bg-neutral-900 border border-red-900/40 hover:border-red-700 transition-colors">
                    <div class="flex items-start justify-between gap-4">
                      <div>
                        <p class="font-semibold text-base leading-snug">{rec.market_question}</p>
                        <button on:click={() => openExternal(`https://polymarket.com/event/${rec.market_slug}`)} class="text-xs text-neutral-500 hover:underline">
                          View on Polymarket ↗
                        </button>
                      </div>
                      <div class="text-right shrink-0">
                        <p class="text-xs text-neutral-400">Bet</p>
                        <p class="text-xl font-bold {rec.suggested_outcome === 'YES' ? 'text-green-400' : 'text-red-400'}">
                          {rec.suggested_outcome}
                        </p>
                        <p class="text-xs font-mono text-neutral-400">{rec.market_price.toFixed(2)}¢</p>
                      </div>
                    </div>
                    <div class="mt-3 flex items-center justify-between text-xs text-neutral-400">
                      <span>Stake ${rec.suggested_budget.toFixed(2)} on {rec.suggested_outcome}</span>
                      <span class="px-2 py-0.5 rounded-full bg-red-900/30 text-red-200">{rec.risk_level}</span>
                    </div>
                    <div class="mt-3 pt-3 border-t border-neutral-800">
                      <p class="text-xs text-neutral-400">{rec.reasoning}</p>
                      <p class="text-[11px] text-neutral-600 mt-2">
                        {rec.source_article_title}
                        {#if rec.source}
                          <span class="ml-1">({rec.source})</span>
                        {/if}
                      </p>
                    </div>
                  </div>
                {/each}
              </div>
            </section>
          {/if}
        </div>
      {:else}
        <div class="h-64 border-2 border-dashed border-neutral-800 rounded-xl flex items-center justify-center text-neutral-500">
          No opportunities found. Try widening the timeframe or increasing the budget.
        </div>
      {/if}
    </div>
  </main>
</div>
