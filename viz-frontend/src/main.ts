import { createState } from "./state";
import { renderTreemap } from "./treemap";
import { initGraphNodes, renderGraph, setGraphFilter, setClusterMode, type GraphFilter, type ClusterMode } from "./graph";
import { setupInteractions } from "./interactions";
import { setupSearch } from "./search";
import { createSummaryBar } from "./summary";
import type { AppState } from "./state";

const renderActiveView = (state: AppState): void => {
  if (state.activeView === "treemap") {
    renderTreemap(state);
  } else {
    renderGraph(state);
  }
};

const init = (): void => {
  const data = window.__FALLOW_DATA__;
  if (!data) {
    document.body.textContent = "Error: No fallow visualization data found.";
    return;
  }

  // Root container
  const app = document.createElement("div");
  app.id = "fallow-app";
  document.body.appendChild(app);

  // Header: summary + controls
  const header = document.createElement("div");
  header.id = "fallow-header";
  app.appendChild(header);

  const summaryEl = createSummaryBar(data.summary, {
    bg: "#ffffff",
    text: "#1a1a1a",
    textSecondary: "#666",
    border: "rgba(0,0,0,0.15)",
    hover: "rgba(255,255,255,0.35)",
    directory: "#E8EDF2",
    directoryText: "#374151",
    statusColors: {
      clean: "#3B82F6",
      hasUnusedExports: "#F59E0B",
      unused: "#EF4444",
      entryPoint: "#10B981",
    },
    tooltipBg: "#ffffff",
    tooltipText: "#1a1a1a",
    tooltipBorder: "#e5e7eb",
    summaryBg: "#f9fafb",
    searchBg: "#ffffff",
    breadcrumbBg: "#f3f4f6",
    breadcrumbText: "#374151",
  });
  header.appendChild(summaryEl);

  // Controls row
  const controls = document.createElement("div");
  controls.id = "fallow-controls";
  header.appendChild(controls);

  // View toggle tabs
  const viewTabs = document.createElement("div");
  viewTabs.id = "fallow-view-tabs";

  const treemapTab = document.createElement("button");
  treemapTab.className = "view-tab active";
  treemapTab.textContent = "Treemap";
  treemapTab.dataset.view = "treemap";
  viewTabs.appendChild(treemapTab);

  const graphTab = document.createElement("button");
  graphTab.className = "view-tab";
  graphTab.textContent = "Graph";
  graphTab.dataset.view = "graph";
  viewTabs.appendChild(graphTab);

  controls.appendChild(viewTabs);

  // Graph filter buttons (visible only in graph view)
  const filterGroup = document.createElement("div");
  filterGroup.id = "fallow-graph-filters";
  filterGroup.style.display = "none";

  const filterButtons: Array<{ label: string; value: GraphFilter }> = [
    { label: "All", value: "all" },
    { label: "Unused", value: "unused" },
    { label: "Issues", value: "issues" },
  ];

  for (const fb of filterButtons) {
    const btn = document.createElement("button");
    btn.className = `filter-btn${fb.value === "all" ? " active" : ""}`;
    btn.textContent = fb.label;
    btn.dataset.filter = fb.value;
    btn.addEventListener("click", () => {
      for (const b of filterGroup.querySelectorAll(".filter-btn")) {
        b.classList.toggle("active", (b as HTMLElement).dataset.filter === fb.value);
      }
      setGraphFilter(state!, fb.value);
    });
    filterGroup.appendChild(btn);
  }
  controls.appendChild(filterGroup);

  // Cluster mode toggle (visible only in graph view)
  const clusterGroup = document.createElement("div");
  clusterGroup.id = "fallow-cluster-mode";
  clusterGroup.style.display = "none";

  const clusterLabel = document.createElement("span");
  clusterLabel.className = "cluster-label";
  clusterLabel.textContent = "Cluster:";
  clusterGroup.appendChild(clusterLabel);

  const clusterButtons: Array<{ label: string; value: ClusterMode }> = [
    { label: "Directory", value: "directory" },
    { label: "Imports", value: "imports" },
  ];

  for (const cb of clusterButtons) {
    const btn = document.createElement("button");
    btn.className = `filter-btn${cb.value === "directory" ? " active" : ""}`;
    btn.textContent = cb.label;
    btn.dataset.cluster = cb.value;
    btn.addEventListener("click", () => {
      for (const b of clusterGroup.querySelectorAll(".filter-btn")) {
        b.classList.toggle("active", (b as HTMLElement).dataset.cluster === cb.value);
      }
      setClusterMode(state!, cb.value);
    });
    clusterGroup.appendChild(btn);
  }
  controls.appendChild(clusterGroup);

  const breadcrumbEl = document.createElement("div");
  breadcrumbEl.id = "fallow-breadcrumb";
  controls.appendChild(breadcrumbEl);

  const canvas = document.createElement("canvas");
  canvas.id = "fallow-canvas";
  app.appendChild(canvas);

  const state = createState(data, canvas);
  if (!state) return;

  // Apply theme to body
  document.body.style.backgroundColor = state.theme.bg;
  document.body.style.color = state.theme.text;
  summaryEl.style.backgroundColor = state.theme.summaryBg;
  breadcrumbEl.style.backgroundColor = state.theme.breadcrumbBg;
  breadcrumbEl.style.color = state.theme.breadcrumbText;

  // Search
  const searchInput = setupSearch(state);
  controls.appendChild(searchInput);

  // Dark mode toggle
  const darkModeBtn = document.createElement("button");
  darkModeBtn.id = "fallow-darkmode";
  darkModeBtn.textContent = state.darkMode ? "☀" : "☾";
  darkModeBtn.title = "Toggle dark mode";
  controls.appendChild(darkModeBtn);

  // Legend
  const legend = document.createElement("div");
  legend.id = "fallow-legend";
  const legendItems: Array<[string, string]> = [
    ["Clean", state.theme.statusColors.clean],
    ["Entry point", state.theme.statusColors.entryPoint],
    ["Unused exports", state.theme.statusColors.hasUnusedExports],
    ["Unused file", state.theme.statusColors.unused],
  ];
  for (const [label, color] of legendItems) {
    const item = document.createElement("span");
    item.className = "legend-item";

    const swatch = document.createElement("span");
    swatch.className = "legend-swatch";
    swatch.style.backgroundColor = color;
    item.appendChild(swatch);

    const text = document.createElement("span");
    text.textContent = label;
    item.appendChild(text);

    legend.appendChild(item);
  }
  controls.appendChild(legend);

  // View tab switching
  let graphInitialized = false;

  const switchView = (view: "treemap" | "graph"): void => {
    state.activeView = view;
    treemapTab.classList.toggle("active", view === "treemap");
    graphTab.classList.toggle("active", view === "graph");
    breadcrumbEl.style.display = view === "treemap" ? "" : "none";
    filterGroup.style.display = view === "graph" ? "" : "none";
    clusterGroup.style.display = view === "graph" ? "" : "none";

    if (view === "graph" && !graphInitialized) {
      graphInitialized = true;
      initGraphNodes(state);
    } else {
      renderActiveView(state);
    }
  };

  treemapTab.addEventListener("click", () => switchView("treemap"));
  graphTab.addEventListener("click", () => switchView("graph"));

  // Initial render
  renderTreemap(state);

  // Wire up interactions
  setupInteractions(state, breadcrumbEl, searchInput, summaryEl, darkModeBtn, renderActiveView);
};

// Boot
if (document.readyState === "loading") {
  document.addEventListener("DOMContentLoaded", init);
} else {
  init();
}
