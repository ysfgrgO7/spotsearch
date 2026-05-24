const { invoke } = window.__TAURI__.core;

let inputEl;
let resultsEl;
let selectedIndex = 0;
let currentResults = [];
let searchId = 0;

function updateVisibility(query) {
  const container = document.getElementById("app");
  if (query && query.trim()) {
    container.classList.add("show-results");
  } else {
    container.classList.remove("show-results");
  }
}

window.addEventListener("DOMContentLoaded", () => {
  inputEl = document.getElementById("search-input");
  resultsEl = document.getElementById("results");

  inputEl.focus();

  let debounceTimer;
  inputEl.addEventListener("input", (e) => {
    updateVisibility(e.target.value);
    clearTimeout(debounceTimer);
    debounceTimer = setTimeout(() => {
      search(e.target.value);
    }, 15); // Ultra-responsive 15ms debounce
  });

  document.addEventListener("keydown", (e) => {
    if (e.key === "Escape") {
      e.preventDefault();
      invoke("hide_window");
    } else if (e.key === "ArrowDown") {
      e.preventDefault();
      if (selectedIndex < currentResults.length - 1) {
        selectedIndex++;
        updateSelection();
      }
    } else if (e.key === "ArrowUp") {
      e.preventDefault();
      if (selectedIndex > 0) {
        selectedIndex--;
        updateSelection();
      }
    } else if (e.key === "Enter") {
      e.preventDefault();
      if (currentResults.length > 0) {
        openResult(currentResults[selectedIndex]);
      }
    }
  });

  // Focus input when window gains focus
  window.addEventListener("focus", () => {
    inputEl.focus();
    inputEl.select();
  });
});

async function search(query) {
  if (!query.trim()) {
    currentResults = [];
    selectedIndex = 0;
    updateVisibility("");
    renderResults();
    return;
  }

  // Increment sequential query ID to discard stale results
  const mySearchId = ++searchId;

  try {
    const results = await invoke("search", { query });
    if (mySearchId === searchId) {
      currentResults = results;
      selectedIndex = 0;
      renderResults();
    }
  } catch (error) {
    console.error("Search error:", error);
  }
}

// SVG icons for file types
const FILE_ICON = `<svg xmlns="http://www.w3.org/2000/svg" width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M13 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V9z"/><polyline points="13 2 13 9 20 9"/></svg>`;
const APP_ICON_FALLBACK = `<svg xmlns="http://www.w3.org/2000/svg" width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><rect x="3" y="3" width="7" height="7"/><rect x="14" y="3" width="7" height="7"/><rect x="14" y="14" width="7" height="7"/><rect x="3" y="14" width="7" height="7"/></svg>`;

function renderResults() {
  resultsEl.innerHTML = "";

  // Append a sliding selection backdrop element
  const backdrop = document.createElement("div");
  backdrop.className = "selection-backdrop";
  resultsEl.appendChild(backdrop);

  if (currentResults.length === 0) {
    updateSelection();
    return;
  }

  const apps = currentResults.filter(r => r.is_app);
  const files = currentResults.filter(r => !r.is_app);

  let flatIndex = 0;

  // --- Applications section ---
  if (apps.length > 0) {
    const header = document.createElement("div");
    header.className = "section-header";
    header.textContent = "Applications";
    resultsEl.appendChild(header);

    apps.forEach((result) => {
      const itemIndex = flatIndex++;
      resultsEl.appendChild(createResultItem(result, itemIndex));
    });
  }

  // --- Files section ---
  if (files.length > 0) {
    const header = document.createElement("div");
    header.className = "section-header";
    header.textContent = "Files";
    resultsEl.appendChild(header);

    files.forEach((result) => {
      const itemIndex = flatIndex++;
      resultsEl.appendChild(createResultItem(result, itemIndex));
    });
  }

  // Setup initial selection & sliding backdrop positions
  updateSelection();
}

function createResultItem(result, idx) {
  const item = document.createElement("div");
  item.className = "result-item";
  item.dataset.index = idx;

  // Icon
  const iconWrap = document.createElement("div");
  iconWrap.className = "result-icon";

  if (result.is_app && result.icon_data) {
    const img = document.createElement("img");
    img.src = result.icon_data;
    img.width = 28;
    img.height = 28;
    img.style.borderRadius = "6px";
    img.onerror = () => { iconWrap.innerHTML = APP_ICON_FALLBACK; };
    iconWrap.appendChild(img);
  } else if (result.is_app) {
    iconWrap.innerHTML = APP_ICON_FALLBACK;
  } else {
    iconWrap.innerHTML = FILE_ICON;
  }
  item.appendChild(iconWrap);

  // Text
  const textContainer = document.createElement("div");
  textContainer.className = "result-text";

  const name = document.createElement("div");
  name.className = "result-name";
  name.textContent = result.name;

  textContainer.appendChild(name);

  if (result.subtitle) {
    const subtitle = document.createElement("div");
    subtitle.className = "result-subtitle";
    // For files, shorten the path by replacing home dir
    let sub = result.subtitle;
    const home = "/home/";
    if (!result.is_app && sub.startsWith(home)) {
      const afterHome = sub.substring(home.length);
      const slashIdx = afterHome.indexOf("/");
      if (slashIdx !== -1) {
        sub = "~" + afterHome.substring(slashIdx);
      }
    }
    subtitle.textContent = sub;
    textContainer.appendChild(subtitle);
  }

  item.appendChild(textContainer);

  // Keyboard and mouse pointer syncing - sliding selection backdrop follows mouse movement
  item.addEventListener("mouseenter", () => {
    selectedIndex = idx;
    updateSelection();
  });

  item.addEventListener("click", () => {
    selectedIndex = idx;
    openResult(result);
  });

  return item;
}

// In-place UI selection updates: completely eliminates DOM rebuilding during navigations!
function updateSelection() {
  const items = resultsEl.querySelectorAll(".result-item");
  let selectedEl = null;

  items.forEach((item) => {
    const idx = parseInt(item.dataset.index, 10);
    if (idx === selectedIndex) {
      item.classList.add("selected");
      selectedEl = item;
    } else {
      item.classList.remove("selected");
    }
  });

  // Smooth sliding backdrop transition
  updateBackdrop(selectedEl);

  // Fast scrolling anchor alignment
  if (selectedEl) {
    selectedEl.scrollIntoView({ block: "nearest" });
  }
}

function updateBackdrop(selectedEl) {
  const backdrop = resultsEl.querySelector(".selection-backdrop");
  if (!backdrop) return;

  if (selectedEl) {
    backdrop.style.opacity = "1";
    backdrop.style.height = `${selectedEl.offsetHeight}px`;
    backdrop.style.transform = `translateY(${selectedEl.offsetTop}px)`;
  } else {
    backdrop.style.opacity = "0";
  }
}

async function openResult(result) {
  try {
    await invoke("open_result", { result });
    inputEl.value = "";
    currentResults = [];
    updateVisibility("");
    renderResults();
  } catch (error) {
    console.error("Open error:", error);
  }
}
