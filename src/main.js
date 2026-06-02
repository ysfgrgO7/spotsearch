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

  // Load custom theme
  invoke("get_config")
    .then((config) => {
      if (config && config.theme) {
        applyTheme(config.theme);
      }
    })
    .catch((err) => {
      console.error("Failed to load custom theme:", err);
    });

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
    } else if (e.key === "," && (e.ctrlKey || e.metaKey)) {
      e.preventDefault();
      invoke("open_settings");
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

  const settingsBtn = document.getElementById("settings-btn");
  if (settingsBtn) {
    settingsBtn.addEventListener("click", () => {
      invoke("open_settings");
    });
  }

  // Focus input when window gains focus
  window.addEventListener("focus", () => {
    inputEl.focus();
    const len = inputEl.value.length;
    inputEl.setSelectionRange(len, len);
  });

  // Keep focus locked to the search input within the window
  document.addEventListener("mousedown", (e) => {
    const settingsBtn = document.getElementById("settings-btn");
    if (e.target !== settingsBtn && !settingsBtn?.contains(e.target)) {
      // Small timeout to allow other click events (like result selection) to register first
      setTimeout(() => {
        if (inputEl) inputEl.focus();
      }, 0);
    }
  });
});

function evaluateMath(str) {
  str = str.replace(/\s+/g, '');
  if (!/^[0-9+\-*/().^%]+$/.test(str)) {
    return null;
  }
  if (!/[+\-*/%^]/.test(str)) {
    return null;
  }

  let index = 0;
  
  function peek() {
    return index < str.length ? str[index] : null;
  }
  
  function consume(char) {
    if (peek() === char) {
      index++;
      return true;
    }
    return false;
  }
  
  function parseExpression() {
    let val = parseTerm();
    if (val === null) return null;
    
    while (true) {
      if (consume('+')) {
        let right = parseTerm();
        if (right === null) return null;
        val += right;
      } else if (consume('-')) {
        let right = parseTerm();
        if (right === null) return null;
        val -= right;
      } else {
        break;
      }
    }
    return val;
  }
  
  function parseTerm() {
    let val = parseFactor();
    if (val === null) return null;
    
    while (true) {
      if (consume('*')) {
        let right = parseFactor();
        if (right === null) return null;
        val *= right;
      } else if (consume('/')) {
        let right = parseFactor();
        if (right === null) return null;
        if (right === 0) return null;
        val /= right;
      } else if (consume('%')) {
        let right = parseFactor();
        if (right === null) return null;
        val %= right;
      } else {
        break;
      }
    }
    return val;
  }
  
  function parseFactor() {
    let val = parseBase();
    if (val === null) return null;
    
    if (consume('^')) {
      let exponent = parseFactor();
      if (exponent === null) return null;
      val = Math.pow(val, exponent);
    }
    return val;
  }
  
  function parseBase() {
    if (consume('(')) {
      let val = parseExpression();
      if (val === null) return null;
      if (!consume(')')) return null;
      return val;
    }
    
    if (consume('-')) {
      let val = parseBase();
      return val === null ? null : -val;
    }
    
    if (consume('+')) {
      return parseBase();
    }
    
    let start = index;
    while (peek() !== null && /[0-9.]/.test(peek())) {
      index++;
    }
    
    if (start === index) {
      return null;
    }
    
    let numStr = str.substring(start, index);
    let val = parseFloat(numStr);
    if (isNaN(val)) return null;
    return val;
  }
  
  try {
    let result = parseExpression();
    if (index === str.length && result !== null && !isNaN(result) && isFinite(result)) {
      return result;
    }
  } catch (e) {}
  return null;
}

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
      
      const calcResult = evaluateMath(query);
      if (calcResult !== null) {
        const formattedResult = calcResult.toLocaleString(undefined, { maximumFractionDigits: 10 });
        const calcItem = {
          name: formattedResult,
          subtitle: `Calculator: ${query}`,
          is_app: false,
          is_calc: true,
          value: calcResult.toString(),
        };
        currentResults.unshift(calcItem);
      }
      
      selectedIndex = 0;
      renderResults();
    }
  } catch (error) {
    console.error("Search error:", error);
    if (mySearchId === searchId) {
      const calcResult = evaluateMath(query);
      if (calcResult !== null) {
        const formattedResult = calcResult.toLocaleString(undefined, { maximumFractionDigits: 10 });
        const calcItem = {
          name: formattedResult,
          subtitle: `Calculator: ${query}`,
          is_app: false,
          is_calc: true,
          value: calcResult.toString(),
        };
        currentResults = [calcItem];
        selectedIndex = 0;
        renderResults();
      }
    }
  }
}

// SVG icons for file types
const FILE_ICON = `<svg xmlns="http://www.w3.org/2000/svg" width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M13 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V9z"/><polyline points="13 2 13 9 20 9"/></svg>`;
const APP_ICON_FALLBACK = `<svg xmlns="http://www.w3.org/2000/svg" width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><rect x="3" y="3" width="7" height="7"/><rect x="14" y="3" width="7" height="7"/><rect x="14" y="14" width="7" height="7"/><rect x="3" y="14" width="7" height="7"/></svg>`;
const CALC_ICON = `<svg xmlns="http://www.w3.org/2000/svg" width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><rect x="4" y="2" width="16" height="20" rx="2" ry="2"/><line x1="8" y1="6" x2="16" y2="6"/><line x1="16" y1="14" x2="16" y2="18"/><path d="M16 10h.01"/><path d="M12 10h.01"/><path d="M8 10h.01"/><path d="M12 14h.01"/><path d="M8 14h.01"/><path d="M12 18h.01"/><path d="M8 18h.01"/></svg>`;
const WEB_ICON = `<svg xmlns="http://www.w3.org/2000/svg" width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="10"/><line x1="2" y1="12" x2="22" y2="12"/><path d="M12 2a15.3 15.3 0 0 1 4 10 15.3 15.3 0 0 1-4 10 15.3 15.3 0 0 1-4-10 15.3 15.3 0 0 1 4-10z"/></svg>`;

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

  const calcs = currentResults.filter(r => r.is_calc);
  const apps = currentResults.filter(r => r.is_app);
  const websearches = currentResults.filter(r => r.is_websearch);
  const files = currentResults.filter(r => !r.is_app && !r.is_calc && !r.is_websearch);

  let flatIndex = 0;

  // --- Calculator section ---
  if (calcs.length > 0) {
    const header = document.createElement("div");
    header.className = "section-header";
    header.textContent = "Calculator";
    resultsEl.appendChild(header);

    calcs.forEach((result) => {
      const itemIndex = flatIndex++;
      resultsEl.appendChild(createResultItem(result, itemIndex));
    });
  }

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

  // --- Web Search section ---
  if (websearches.length > 0) {
    const header = document.createElement("div");
    header.className = "section-header";
    header.textContent = "Web Search";
    resultsEl.appendChild(header);

    websearches.forEach((result) => {
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

  if (result.is_calc) {
    iconWrap.innerHTML = CALC_ICON;
  } else if (result.is_websearch) {
    iconWrap.innerHTML = WEB_ICON;
  } else if (result.is_app && result.icon_data) {
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

  // Click to select, second click to open (hover selection removed)
  item.addEventListener("click", () => {
    if (selectedIndex === idx) {
      openResult(result);
    } else {
      selectedIndex = idx;
      updateSelection();
    }
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
  if (result.is_calc) {
    try {
      await navigator.clipboard.writeText(result.value);
    } catch (error) {
      console.error("Failed to copy to clipboard:", error);
    }
    inputEl.value = "";
    currentResults = [];
    updateVisibility("");
    renderResults();
    await invoke("hide_window");
    return;
  }

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

function applyTheme(colors) {
  if (!colors) return;
  const root = document.documentElement;
  root.style.setProperty('--bg-color', colors.bg_color || '#2b2b2b');
  root.style.setProperty('--text-color', colors.text_color || '#f4f4f5');
  root.style.setProperty('--text-dim', colors.text_dim || '#a1a1aa');
  root.style.setProperty('--accent-bg', colors.accent_bg || 'rgba(139, 92, 246, 0.15)');
  root.style.setProperty('--accent-bar', colors.accent_bar || '#8560f6');
  root.style.setProperty('--glow-color', colors.glow_color || 'rgba(139, 92, 246, 0.12)');
}
