<script lang="ts">
  let budget = 100;
  let timeframe = "1w";
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

      <!-- Card 1: Budget -->
      <div class="p-5 rounded-xl bg-neutral-900 border border-neutral-800 shadow-sm">
        <label for="budget-input" class="block text-sm font-medium text-neutral-400 mb-2">Capital Allocation ($)</label>
        <input
          id="budget-input"
          type="number"
          bind:value={budget}
          class="w-full bg-neutral-950 border border-neutral-800 rounded-lg px-4 py-3 text-lg focus:outline-none focus:ring-2 focus:ring-emerald-500/50 transition-all"
        />
      </div>

      <!-- Card 2: Timeframe -->
      <div class="p-5 rounded-xl bg-neutral-900 border border-neutral-800 shadow-sm">
        <label for="timeframe-select" class="block text-sm font-medium text-neutral-400 mb-2">Time Horizon</label>
        <select
          id="timeframe-select"
          bind:value={timeframe}
          class="w-full bg-neutral-950 border border-neutral-800 rounded-lg px-4 py-3 text-lg focus:outline-none focus:ring-2 focus:ring-emerald-500/50 transition-all appearance-none"
        >
          <option value="24h">24 Hours (Day Trade)</option>
          <option value="1w">1 Week (Swing)</option>
          <option value="1m">1 Month (Mid-term)</option>
          <option value="1y">1 Year (Long-term)</option>
        </select>
      </div>

      <!-- Card 3: Action -->
      <div class="flex items-end">
        <button class="w-full h-13 bg-emerald-600 hover:bg-emerald-500 text-white font-semibold rounded-xl shadow-lg shadow-emerald-900/20 transition-all active:scale-95">
          Run Analysis
        </button>
      </div>
    </div>

    <!-- PLACEHOLDER RESULTS AREA -->
    <div class="border-t border-neutral-800 pt-8">
      <div class="h-64 border-2 border-dashed border-neutral-800 rounded-xl flex items-center justify-center text-neutral-500">
        Results will appear here...
      </div>
    </div>
  </main>
</div>