const { invoke } = window.__TAURI__.core;

let currentConfig = {
  theme: {
    bg_color: "#2b2b2b",
    text_color: "#f4f4f5",
    text_dim: "#a1a1aa",
    accent_bg: "rgba(139, 92, 246, 0.15)",
    accent_bar: "#8560f6",
    glow_color: "rgba(139, 92, 246, 0.12)"
  },
  search_paths: [],
  excluded_dirs: [],
  excluded_extensions: [],
  max_depth: 7
};

// UI Elements
const tabs = document.querySelectorAll(".tab-btn");
const tabContents = document.querySelectorAll(".tab-content");
const depthInput = document.getElementById("depth-input");
const depthVal = document.getElementById("depth-val");
const hideOnBlurInput = document.getElementById("hide-on-blur-input");
const pathsList = document.getElementById("paths-list");
const newPathInput = document.getElementById("new-path-input");
const addPathBtn = document.getElementById("add-path-btn");
const excludedDirsContainer = document.getElementById("excluded-dirs-container");
const newDirInput = document.getElementById("new-dir-input");
const addDirBtn = document.getElementById("add-dir-btn");
const excludedExtsContainer = document.getElementById("excluded-exts-container");
const newExtInput = document.getElementById("new-ext-input");
const addExtBtn = document.getElementById("add-ext-btn");
const saveBtn = document.getElementById("save-btn");
const cancelBtn = document.getElementById("cancel-btn");
const statusMsg = document.getElementById("status-msg");

// Color Pickers
const bgPicker = document.getElementById("color-bg");
const textPicker = document.getElementById("color-text");
const dimPicker = document.getElementById("color-dim");
const accentPicker = document.getElementById("color-accent");

// Updates UI Elements
const currentVersionDisplay = document.getElementById("current-version-display");
const latestVersionDisplay = document.getElementById("latest-version-display");
const updateMessage = document.getElementById("update-message");
const checkUpdatesBtn = document.getElementById("check-updates-btn");
const applyUpdateBtn = document.getElementById("apply-update-btn");

// DOM Initialized
window.addEventListener("DOMContentLoaded", async () => {
  setupTabs();
  setupEventListeners();
  await loadConfig();
  await checkCurrentVersion();
});

async function checkCurrentVersion() {
  try {
    const updateInfo = await invoke("check_for_updates");
    if (updateInfo) {
      currentVersionDisplay.textContent = updateInfo.current_version;
    }
  } catch (error) {
    console.error("Failed to load version info:", error);
  }
}

// Sidebar Tab Setup
function setupTabs() {
  tabs.forEach(tab => {
    tab.addEventListener("click", () => {
      tabs.forEach(t => t.classList.remove("active"));
      tabContents.forEach(c => c.classList.remove("active"));

      tab.classList.add("active");
      const activeTabId = tab.dataset.tab;
      document.getElementById(activeTabId).classList.add("active");
    });
  });
}

// Load current configuration from backend
async function loadConfig() {
  try {
    currentConfig = await invoke("get_config");
    
    // Fill General inputs
    depthInput.value = currentConfig.max_depth;
    depthVal.textContent = currentConfig.max_depth;
    hideOnBlurInput.checked = currentConfig.hide_on_blur !== false;

    // Fill Appearance inputs
    if (currentConfig.theme) {
      bgPicker.value = currentConfig.theme.bg_color;
      bgPicker.nextElementSibling.textContent = currentConfig.theme.bg_color;

      textPicker.value = currentConfig.theme.text_color;
      textPicker.nextElementSibling.textContent = currentConfig.theme.text_color;

      dimPicker.value = currentConfig.theme.text_dim;
      dimPicker.nextElementSibling.textContent = currentConfig.theme.text_dim;

      accentPicker.value = currentConfig.theme.accent_bar;
      accentPicker.nextElementSibling.textContent = currentConfig.theme.accent_bar;

      // Apply theme to settings window immediately on load
      applyTheme(currentConfig.theme);
    }

    // Render list views
    renderPaths();
    renderExcludedDirs();
    renderExcludedExts();
  } catch (error) {
    showStatus("Failed to load settings: " + error, "error");
  }
}

// Render Directories List
function renderPaths() {
  pathsList.innerHTML = "";
  if (currentConfig.search_paths.length === 0) {
    pathsList.innerHTML = `<div style="padding: 16px; text-align: center; color: var(--text-muted); font-size: 13px;">No search directories configured.</div>`;
    return;
  }

  currentConfig.search_paths.forEach((path, index) => {
    const item = document.createElement("div");
    item.className = "path-item";

    const pathText = document.createElement("span");
    pathText.className = "path-text";
    pathText.textContent = path;
    pathText.title = path;
    item.appendChild(pathText);

    const deleteBtn = document.createElement("button");
    deleteBtn.className = "delete-btn";
    deleteBtn.innerHTML = `<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><polyline points="3 6 5 6 21 6"/><path d="M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6m3 0V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2"/><line x1="10" y1="11" x2="10" y2="17"/><line x1="14" y1="11" x2="14" y2="17"/></svg>`;
    deleteBtn.addEventListener("click", () => {
      currentConfig.search_paths.splice(index, 1);
      renderPaths();
    });
    item.appendChild(deleteBtn);

    pathsList.appendChild(item);
  });
}

// Render Excluded Folders Pills
function renderExcludedDirs() {
  excludedDirsContainer.innerHTML = "";
  currentConfig.excluded_dirs.forEach((dir, index) => {
    const tag = document.createElement("div");
    tag.className = "tag";
    tag.textContent = dir;

    const remove = document.createElement("span");
    remove.className = "tag-remove";
    remove.innerHTML = "&times;";
    remove.addEventListener("click", () => {
      currentConfig.excluded_dirs.splice(index, 1);
      renderExcludedDirs();
    });
    tag.appendChild(remove);

    excludedDirsContainer.appendChild(tag);
  });
}

// Render Excluded Extensions Pills
function renderExcludedExts() {
  excludedExtsContainer.innerHTML = "";
  currentConfig.excluded_extensions.forEach((ext, index) => {
    const tag = document.createElement("div");
    tag.className = "tag";
    tag.textContent = `.${ext}`;

    const remove = document.createElement("span");
    remove.className = "tag-remove";
    remove.innerHTML = "&times;";
    remove.addEventListener("click", () => {
      currentConfig.excluded_extensions.splice(index, 1);
      renderExcludedExts();
    });
    tag.appendChild(remove);

    excludedExtsContainer.appendChild(tag);
  });
}

// Helper to convert hex to rgba strings
function hexToRgba(hex, alpha) {
  hex = hex.replace('#', '');
  if (hex.length === 3) {
    hex = hex[0] + hex[0] + hex[1] + hex[1] + hex[2] + hex[2];
  }
  const r = parseInt(hex.substring(0, 2), 16);
  const g = parseInt(hex.substring(2, 4), 16);
  const b = parseInt(hex.substring(4, 6), 16);
  return `rgba(${r}, ${g}, ${b}, ${alpha})`;
}

// Dynamic CSS Theme Variable Replacer
function applyTheme(colors) {
  if (!colors) return;
  const root = document.documentElement;
  root.style.setProperty('--bg-color', colors.bg_color);
  root.style.setProperty('--text-color', colors.text_color);
  root.style.setProperty('--text-dim', colors.text_dim);
  root.style.setProperty('--accent-bg', colors.accent_bg);
  root.style.setProperty('--accent-bar', colors.accent_bar);
  root.style.setProperty('--glow-color', colors.glow_color);
}

// Show Toast Status Messages
function showStatus(msg, type) {
  statusMsg.textContent = msg;
  statusMsg.className = "status-msg " + type;
  setTimeout(() => {
    statusMsg.className = "status-msg";
  }, 4000);
}

// Hook all form actions
function setupEventListeners() {
  // Range Depth Change listener
  depthInput.addEventListener("input", (e) => {
    depthVal.textContent = e.target.value;
    currentConfig.max_depth = parseInt(e.target.value, 10);
  });

  // Hide on Blur checkbox listener
  hideOnBlurInput.addEventListener("change", (e) => {
    currentConfig.hide_on_blur = e.target.checked;
  });

  // Bind Color pickers with Hex label updating and live-preview rendering
  function registerColorPicker(picker, key) {
    picker.addEventListener("input", (e) => {
      const val = e.target.value;
      picker.nextElementSibling.textContent = val;
      
      if (!currentConfig.theme) {
        currentConfig.theme = {};
      }
      
      currentConfig.theme[key] = val;
      
      // Auto-compute opacities for the Accent color
      if (key === "accent_bar") {
        currentConfig.theme.accent_bg = hexToRgba(val, 0.15);
        currentConfig.theme.glow_color = hexToRgba(val, 0.12);
        
        // Update root variables of settings sliders/active components immediately
        document.documentElement.style.setProperty('--accent', val);
        document.documentElement.style.setProperty('--accent-light', hexToRgba(val, 0.15));
      }

      // Live Theme Swap Preview!
      applyTheme(currentConfig.theme);
    });
  }

  registerColorPicker(bgPicker, "bg_color");
  registerColorPicker(textPicker, "text_color");
  registerColorPicker(dimPicker, "text_dim");
  registerColorPicker(accentPicker, "accent_bar");

  // Add search path
  addPathBtn.addEventListener("click", () => {
    const val = newPathInput.value.trim();
    if (val) {
      if (!currentConfig.search_paths.includes(val)) {
        currentConfig.search_paths.push(val);
        renderPaths();
      }
      newPathInput.value = "";
    }
  });

  newPathInput.addEventListener("keypress", (e) => {
    if (e.key === "Enter") {
      addPathBtn.click();
    }
  });

  // Add excluded directory
  addDirBtn.addEventListener("click", () => {
    const val = newDirInput.value.trim();
    if (val) {
      if (!currentConfig.excluded_dirs.includes(val)) {
        currentConfig.excluded_dirs.push(val);
        renderExcludedDirs();
      }
      newDirInput.value = "";
    }
  });

  newDirInput.addEventListener("keypress", (e) => {
    if (e.key === "Enter") {
      addDirBtn.click();
    }
  });

  // Add excluded extension
  addExtBtn.addEventListener("click", () => {
    let val = newExtInput.value.trim();
    if (val) {
      if (val.startsWith(".")) {
        val = val.substring(1);
      }
      if (!currentConfig.excluded_extensions.includes(val)) {
        currentConfig.excluded_extensions.push(val);
        renderExcludedExts();
      }
      newExtInput.value = "";
    }
  });

  newExtInput.addEventListener("keypress", (e) => {
    if (e.key === "Enter") {
      addExtBtn.click();
    }
  });

  // Save Config Call
  saveBtn.addEventListener("click", async () => {
    try {
      // Call Rust backend to save
      await invoke("save_config", { newConfig: currentConfig });
      showStatus("Settings saved successfully!", "success");
      
      // Close window after small delay so user sees success message
      setTimeout(async () => {
        const { getCurrentWindow } = window.__TAURI__.window;
        getCurrentWindow().close();
      }, 1000);
    } catch (error) {
      showStatus("Failed to save settings: " + error, "error");
    }
  });

  // Cancel / Close settings
  cancelBtn.addEventListener("click", () => {
    const { getCurrentWindow } = window.__TAURI__.window;
    getCurrentWindow().close();
  });

  // Check for updates button
  checkUpdatesBtn.addEventListener("click", async () => {
    checkUpdatesBtn.disabled = true;
    checkUpdatesBtn.textContent = "Checking...";
    updateMessage.textContent = "Connecting to local repository to inspect updates...";
    updateMessage.style.color = "var(--text-dim)";
    
    try {
      const updateInfo = await invoke("check_for_updates");
      latestVersionDisplay.textContent = updateInfo.latest_version;
      
      if (updateInfo.has_update) {
        updateMessage.textContent = `A newer version (v${updateInfo.latest_version}) is available in your local repository! Click 'Install Update Now' to rebuild and upgrade automatically.`;
        updateMessage.style.color = "#c084fc"; // nice accent color
        applyUpdateBtn.style.display = "block";
      } else {
        updateMessage.textContent = `SpotSearch is up-to-date (v${updateInfo.current_version}). No updates available.`;
        updateMessage.style.color = "#10b981"; // success green
        applyUpdateBtn.style.display = "none";
      }
    } catch (error) {
      updateMessage.textContent = `Update check failed: ${error}. Make sure the repository exists and install.sh was run first.`;
      updateMessage.style.color = "#ef4444"; // error red
      applyUpdateBtn.style.display = "none";
    } finally {
      checkUpdatesBtn.disabled = false;
      checkUpdatesBtn.textContent = "Check for Updates";
    }
  });

  // Apply update button
  applyUpdateBtn.addEventListener("click", async () => {
    applyUpdateBtn.disabled = true;
    applyUpdateBtn.textContent = "Installing Update...";
    updateMessage.textContent = "Rebuilding SpotSearch from local repository. This window will automatically restart in a few moments...";
    updateMessage.style.color = "var(--text-dim)";
    
    try {
      await invoke("apply_update");
    } catch (error) {
      updateMessage.textContent = `Failed to apply update: ${error}`;
      updateMessage.style.color = "#ef4444";
      applyUpdateBtn.disabled = false;
      applyUpdateBtn.textContent = "Install Update Now";
    }
  });
}
