const { invoke } = window.__TAURI__.core;
const { emit } = window.__TAURI__.event;

let currentConfig = {
  theme: {
    bg_color: "#2b2b2b",
    text_color: "#f4f4f5",
    text_dim: "#a1a1aa",
    accent_bg: "rgba(139, 92, 246, 0.15)",
    accent_bar: "#8560f6",
    glow_color: "rgba(139, 92, 246, 0.12)",
    border_radius: 28,
    backdrop_blur: 0
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
const shortcutInput = document.getElementById("shortcut-input");
const resetShortcutBtn = document.getElementById("reset-shortcut-btn");
const webBrowserInput = document.getElementById("web-browser-input");
const webSearchTemplateInput = document.getElementById("web-search-template-input");
const terminalInput = document.getElementById("terminal-input");
const terminalAppsInput = document.getElementById("terminal-apps-input");
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

// Advanced Styling
const radiusInput = document.getElementById("radius-input");
const radiusVal = document.getElementById("radius-val");
const blurInput = document.getElementById("blur-input");
const blurVal = document.getElementById("blur-val");

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
      
      if (activeTabId === "ai") {
        checkAgyStatus();
      }
    });
  });
}

async function checkAgyStatus() {
  const iconEl = document.getElementById("agy-status-icon");
  const textEl = document.getElementById("agy-status-text");
  const hintEl = document.getElementById("agy-status-hint");
  
  iconEl.innerHTML = '<svg xmlns="http://www.w3.org/2000/svg" width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="var(--text-dim)" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M12 2v20M17 5H9.5a3.5 3.5 0 0 0 0 7h5a3.5 3.5 0 0 1 0 7H6"/></svg>';
  textEl.textContent = "Checking status...";
  textEl.style.color = "var(--text-color)";
  hintEl.textContent = "";

  try {
    const isInstalled = await invoke("check_agy_status");
    if (isInstalled) {
      iconEl.innerHTML = '<svg xmlns="http://www.w3.org/2000/svg" width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="#22c55e" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M22 11.08V12a10 10 0 1 1-5.93-9.14"></path><polyline points="22 4 12 14.01 9 11.01"></polyline></svg>';
      textEl.textContent = "Installed and Ready";
      textEl.style.color = "#22c55e";
    } else {
      iconEl.innerHTML = '<svg xmlns="http://www.w3.org/2000/svg" width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="#ef4444" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="10"></circle><line x1="12" y1="8" x2="12" y2="12"></line><line x1="12" y1="16" x2="12.01" y2="16"></line></svg>';
      textEl.textContent = "Not Installed";
      textEl.style.color = "#ef4444";
      hintEl.textContent = "agy is required for SpotSearch AI. Click 'Visit Website' to install it.";
    }
  } catch (err) {
    iconEl.innerHTML = '<svg xmlns="http://www.w3.org/2000/svg" width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="#ef4444" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="10"></circle><line x1="12" y1="8" x2="12" y2="12"></line><line x1="12" y1="16" x2="12.01" y2="16"></line></svg>';
    textEl.textContent = "Error checking status";
    textEl.style.color = "#ef4444";
  }
}

// Load current configuration from backend
async function loadConfig() {
  try {
    currentConfig = await invoke("get_config");
    
    // Fill General inputs
    depthInput.value = currentConfig.max_depth;
    depthVal.textContent = currentConfig.max_depth;
    hideOnBlurInput.checked = currentConfig.hide_on_blur !== false;
    shortcutInput.value = currentConfig.shortcut || "Alt+Shift+Space";
    webBrowserInput.value = currentConfig.web_browser || "default";
    webSearchTemplateInput.value = currentConfig.web_search_template || "https://www.google.com/search?q={query}";
    terminalInput.value = currentConfig.terminal || "default";
    terminalAppsInput.value = (currentConfig.terminal_apps || []).join(", ");

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

      if (currentConfig.theme.border_radius !== undefined) {
        radiusInput.value = currentConfig.theme.border_radius;
        radiusVal.textContent = currentConfig.theme.border_radius;
      }
      if (currentConfig.theme.backdrop_blur !== undefined) {
        blurInput.value = currentConfig.theme.backdrop_blur;
        blurVal.textContent = currentConfig.theme.backdrop_blur;
      }

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

  if (colors.border_radius !== undefined) {
    root.style.setProperty('--app-radius', `${colors.border_radius}px`);
  }
  if (colors.backdrop_blur !== undefined) {
    // Adding a fallback if transparent backgrounds are not supported, but usually ok
    const bgWithAlpha = hexToRgba(colors.bg_color, colors.backdrop_blur > 0 ? 0.75 : 1.0);
    root.style.setProperty('--bg-color', colors.backdrop_blur > 0 ? bgWithAlpha : colors.bg_color);
    root.style.setProperty('--backdrop-blur', `${colors.backdrop_blur}px`);
  }
  
  if (typeof emit === 'function') {
    emit("theme-changed", colors);
  }
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

  // Advanced Styling listeners
  radiusInput.addEventListener("input", (e) => {
    radiusVal.textContent = e.target.value;
    if (!currentConfig.theme) currentConfig.theme = {};
    currentConfig.theme.border_radius = parseInt(e.target.value, 10);
    applyTheme(currentConfig.theme);
  });

  blurInput.addEventListener("input", (e) => {
    blurVal.textContent = e.target.value;
    if (!currentConfig.theme) currentConfig.theme = {};
    currentConfig.theme.backdrop_blur = parseInt(e.target.value, 10);
    applyTheme(currentConfig.theme);
  });

  // Hide on Blur checkbox listener
  hideOnBlurInput.addEventListener("change", (e) => {
    currentConfig.hide_on_blur = e.target.checked;
  });

  // Shortcut recording logic
  let isRecordingShortcut = false;

  shortcutInput.addEventListener("click", async () => {
    isRecordingShortcut = true;
    await invoke("unregister_shortcut");
    shortcutInput.value = "Recording... Press keys";
    shortcutInput.style.borderColor = "var(--accent-bar)";
    shortcutInput.focus();
  });

  shortcutInput.addEventListener("blur", async () => {
    if (isRecordingShortcut) {
      isRecordingShortcut = false;
      await invoke("register_shortcut");
      shortcutInput.value = currentConfig.shortcut || "Alt+Shift+Space";
      shortcutInput.style.borderColor = "";
    }
  });

  shortcutInput.addEventListener("keydown", async (e) => {
    if (!isRecordingShortcut) return;
    e.preventDefault();
    e.stopPropagation();
    e.stopImmediatePropagation();
    
    let keys = [];
    if (e.metaKey || e.key === "Meta" || e.key === "Super" || e.key === "OS") keys.push("Super");
    if (e.ctrlKey || e.key === "Control") keys.push("Control");
    if (e.altKey || e.key === "Alt") keys.push("Alt");
    if (e.shiftKey || e.key === "Shift") keys.push("Shift");
    
    // Ensure unique keys just in case
    keys = [...new Set(keys)];
    
    let key = e.key;
    
    // If only modifier is pressed, update input to show feedback
    if (["Control", "Shift", "Alt", "Meta", "Super", "OS"].includes(key)) {
      shortcutInput.value = keys.join("+") + "+";
      return;
    }
    
    if (key === " ") key = "space";
    else if (key === "Escape") {
      isRecordingShortcut = false;
      await invoke("register_shortcut");
      shortcutInput.value = currentConfig.shortcut || "Alt+Shift+Space";
      shortcutInput.style.borderColor = "";
      return;
    } else if (key.length === 1) {
      key = key.toLowerCase();
    }
    
    keys.push(key);
    const shortcutStr = keys.join("+");
    
    if (keys.includes("Super")) {
      alert("Error: Keybindings starting with Super are not allowed.");
      shortcutInput.value = "Super key not allowed";
      setTimeout(() => {
        if (isRecordingShortcut) {
          shortcutInput.value = "Recording... Press keys";
        }
      }, 1000);
      return;
    }
    
    // Check if at least one modifier key is present
    if (!["Control", "Alt", "Shift", "Super"].some(mod => keys.includes(mod))) {
      shortcutInput.value = "Requires MOD key";
      setTimeout(() => {
        if (isRecordingShortcut) {
          shortcutInput.value = "Recording... Press keys";
        }
      }, 1000);
      return;
    }
    
    shortcutInput.value = shortcutStr;
    currentConfig.shortcut = shortcutStr;
    isRecordingShortcut = false;
    await invoke("register_shortcut");
    shortcutInput.style.borderColor = "";
    shortcutInput.blur();
  });

  resetShortcutBtn.addEventListener("click", () => {
    currentConfig.shortcut = "Alt+Shift+Space";
    shortcutInput.value = "Alt+Shift+Space";
  });

  // Web Browser input change listener
  webBrowserInput.addEventListener("input", (e) => {
    currentConfig.web_browser = e.target.value.trim();
  });

  // Web Search Template input change listener
  webSearchTemplateInput.addEventListener("input", (e) => {
    currentConfig.web_search_template = e.target.value.trim();
  });

  // Terminal input change listener
  terminalInput.addEventListener("input", (e) => {
    currentConfig.terminal = e.target.value.trim();
  });

  // Terminal Apps input change listener
  terminalAppsInput.addEventListener("input", (e) => {
    currentConfig.terminal_apps = e.target.value.split(",")
      .map(app => app.trim())
      .filter(app => app.length > 0);
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
  const { listen } = window.__TAURI__.event;
  let logHistory = [];
  let currentProgress = 0;

  const progressContainer = document.getElementById("update-progress-container");
  const progressStatus = document.getElementById("progress-status");
  const progressPercent = document.getElementById("progress-percent");
  const progressBarFill = document.getElementById("progress-bar-fill");
  const logConsole = document.getElementById("update-log-console");
  const copyLogBtn = document.getElementById("copy-log-btn");

  function appendLog(text, isSystem = false, isError = false) {
    const line = document.createElement("div");
    line.className = "log-line";
    if (isSystem) line.classList.add("system-line");
    if (isError) line.classList.add("error-line");
    line.textContent = text;
    logConsole.appendChild(line);
    logConsole.scrollTop = logConsole.scrollHeight;
    logHistory.push(text);
  }

  function setProgress(percentage, statusText) {
    const pct = Math.min(Math.max(Math.round(percentage), 0), 100);
    progressBarFill.style.width = `${pct}%`;
    progressPercent.textContent = `${pct}%`;
    if (statusText) {
      progressStatus.textContent = statusText;
    }
  }

  function updateProgressFromLog(line) {
    if (line.includes("Starting update process")) {
      currentProgress = 5;
      setProgress(currentProgress, "Initializing updater...");
    } else if (line.includes("Pulling latest changes")) {
      currentProgress = 10;
      setProgress(currentProgress, "Pulling latest git changes...");
    } else if (line.includes("Git pull completed") || line.includes("Warning: Git pull failed")) {
      currentProgress = 15;
      setProgress(currentProgress, "Git sync completed.");
    } else if (line.includes("Running install script")) {
      currentProgress = 20;
      setProgress(currentProgress, "Running installer...");
    } else if (line.includes("Updating NPM dependencies")) {
      currentProgress = 25;
      setProgress(currentProgress, "Updating NPM dependencies...");
    } else if (line.includes("Building SpotSearch in release mode")) {
      currentProgress = 40;
      setProgress(currentProgress, "Compiling Tauri application...");
    } else if (line.includes("Compiling ")) {
      if (currentProgress < 88) {
        currentProgress += 0.5;
        setProgress(currentProgress, "Compiling application...");
      }
    } else if (line.includes("Setting up directories")) {
      currentProgress = 90;
      setProgress(currentProgress, "Configuring directories...");
    } else if (line.includes("Installing SpotSearch binary")) {
      currentProgress = 93;
      setProgress(currentProgress, "Installing executable...");
    } else if (line.includes("Registering desktop launcher")) {
      currentProgress = 96;
      setProgress(currentProgress, "Registering desktop launcher...");
    } else if (line.includes("Auto-update successfully built and installed")) {
      currentProgress = 98;
      setProgress(currentProgress, "Build and install completed!");
    }
  }

  // Copy Logs button
  copyLogBtn.addEventListener("click", () => {
    navigator.clipboard.writeText(logHistory.join("\n"))
      .then(() => {
        copyLogBtn.textContent = "Copied!";
        setTimeout(() => {
          copyLogBtn.textContent = "Copy Logs";
        }, 2000);
      })
      .catch(err => {
        console.error("Failed to copy logs:", err);
      });
  });

  // Listen to background progress events
  listen("update-log", (event) => {
    const payload = event.payload;
    appendLog(payload.message, payload.is_system);
    updateProgressFromLog(payload.message);
  });

  listen("update-error", (event) => {
    appendLog(`[Error] ${event.payload}`, true, true);
    setProgress(currentProgress, "Update failed.");
    progressBarFill.style.background = "var(--danger)";
    progressBarFill.style.boxShadow = "0 0 10px rgba(239, 68, 68, 0.5)";
    applyUpdateBtn.disabled = false;
    checkUpdatesBtn.disabled = false;
    applyUpdateBtn.textContent = "Retry Update";
  });

  listen("update-complete", () => {
    appendLog("[System] Update process finished successfully!", true);
    setProgress(100, "Update complete! Restarting SpotSearch...");
    progressBarFill.style.background = "linear-gradient(90deg, #34d399, #10b981)";
    progressBarFill.style.boxShadow = "0 0 10px rgba(52, 211, 153, 0.5)";
  });

  applyUpdateBtn.addEventListener("click", async () => {
    applyUpdateBtn.disabled = true;
    checkUpdatesBtn.disabled = true;
    applyUpdateBtn.textContent = "Updating...";
    
    // Show progress panel & reset state
    progressContainer.style.display = "flex";
    logConsole.innerHTML = "";
    logHistory = [];
    currentProgress = 0;
    setProgress(0, "Starting update...");
    progressBarFill.style.background = "linear-gradient(90deg, var(--accent), #c084fc)";
    progressBarFill.style.boxShadow = "0 0 10px rgba(133, 96, 246, 0.5)";
    
    updateMessage.textContent = "Rebuilding SpotSearch from local repository. Track real-time progress below:";
    updateMessage.style.color = "var(--text-dim)";
    
    try {
      await invoke("apply_update");
    } catch (error) {
      appendLog(`[Error] Failed to trigger update: ${error}`, true, true);
      updateMessage.textContent = `Failed to apply update: ${error}`;
      updateMessage.style.color = "#ef4444";
      applyUpdateBtn.disabled = false;
      checkUpdatesBtn.disabled = false;
      applyUpdateBtn.textContent = "Install Update Now";
    }
  });
}
