const { invoke } = window.__TAURI__.core;
const { listen } = window.__TAURI__.event;

// UI State variables
let displays = [];
let profiles = [];
let activeProfileId = null; // Current index of profile being edited
let activeProfileTabDisplayId = null; // Current display ID tab in editor

// Elements cache
let panels = {};
let navItems = [];
let displaysContainer;
let profilesListContainer;
let profileEditor;
let profileEditorPlaceholder;
let profileForm;
let btnAddProfile;
let btnDeleteProfile;
let btnCancelEdit;
let btnApplyManual;
let btnRefreshDisplays;
let btnGlobalReset;
let toggleDaemon;
let daemonDot;
let daemonStatusText;
let activeProfileBanner;
let activeProfileName;
let displayTabsHeader;
let displayTabsContent;

// Navigation Panel handling
function initNavigation() {
  navItems = document.querySelectorAll(".nav-item");
  panels = {
    dashboard: document.getElementById("panel-dashboard"),
    profiles: document.getElementById("panel-profiles"),
    settings: document.getElementById("panel-settings")
  };

  navItems.forEach(item => {
    item.addEventListener("click", () => {
      const target = item.getAttribute("data-target");
      
      // Update nav active class
      navItems.forEach(nav => nav.classList.remove("active"));
      item.classList.add("active");

      // Show target panel
      Object.keys(panels).forEach(key => {
        if (key === target) {
          panels[key].classList.add("active");
        } else {
          panels[key].classList.remove("active");
        }
      });

      // Context refreshers
      if (target === "dashboard") {
        refreshDisplays();
      } else if (target === "profiles") {
        refreshProfilesList();
      } else if (target === "settings") {
        loadGlobalSettings();
      }
    });
  });
}

// Custom Toast Helper
function showToast(message, type = "info") {
  const container = document.getElementById("toast-container");
  const toast = document.createElement("div");
  toast.className = `toast toast-${type}`;
  
  let iconSvg = "";
  if (type === "success") {
    iconSvg = `<svg style="width:14px;height:14px;fill:none;stroke:currentColor;stroke-width:2.5" viewBox="0 0 24 24"><polyline points="20 6 9 17 4 12"/></svg>`;
  } else if (type === "error") {
    iconSvg = `<svg style="width:14px;height:14px;fill:none;stroke:currentColor;stroke-width:2.5" viewBox="0 0 24 24"><line x1="18" y1="6" x2="6" y2="18"/><line x1="6" y1="6" x2="18" y2="18"/></svg>`;
  } else {
    iconSvg = `<svg style="width:14px;height:14px;fill:none;stroke:currentColor;stroke-width:2" viewBox="0 0 24 24"><circle cx="12" cy="12" r="10"/><line x1="12" y1="16" x2="12" y2="12"/><line x1="12" y1="8" x2="12.01" y2="8"/></svg>`;
  }

  toast.innerHTML = `${iconSvg} <span>${message}</span>`;
  container.appendChild(toast);

  // Trigger transition
  setTimeout(() => toast.classList.add("show"), 10);

  // Auto remove
  setTimeout(() => {
    toast.classList.remove("show");
    setTimeout(() => toast.remove(), 300);
  }, 3000);
}

// Custom Confirmation Dialog Helper
function customConfirm(title, message) {
  return new Promise((resolve) => {
    const modal = document.getElementById("confirm-modal");
    const titleEl = document.getElementById("confirm-title");
    const messageEl = document.getElementById("confirm-message");
    const btnCancel = document.getElementById("confirm-cancel");
    const btnOk = document.getElementById("confirm-ok");

    titleEl.textContent = title;
    messageEl.textContent = message;
    
    modal.style.display = "flex";
    setTimeout(() => modal.classList.add("show"), 10);

    const cleanup = (result) => {
      modal.classList.remove("show");
      setTimeout(() => {
        modal.style.display = "none";
      }, 200);
      btnCancel.removeEventListener("click", onCancel);
      btnOk.removeEventListener("click", onOk);
      resolve(result);
    };

    const onCancel = () => cleanup(false);
    const onOk = () => cleanup(true);

    btnCancel.addEventListener("click", onCancel);
    btnOk.addEventListener("click", onOk);
  });
}

// Refresh connected monitors list (Dashboard)
async function refreshDisplays() {
  try {
    displays = await invoke("get_displays");
    renderDisplayCards();
  } catch (err) {
    console.error("Failed to load displays:", err);
    showToast("Failed to detect connected monitors.", "error");
  }
}

// Render display cards on Dashboard
function renderDisplayCards() {
  displaysContainer.innerHTML = "";
  
  if (displays.length === 0) {
    displaysContainer.innerHTML = "<p class='text-muted'>No displays detected.</p>";
    return;
  }

  displays.forEach(d => {
    const card = document.createElement("div");
    card.className = "display-card";

    // Primary tag
    const primaryTag = d.is_primary ? "<span class='badge'>Primary</span>" : "";

    // Build Resolution options
    let resOptions = d.supported_resolutions.map(r => {
      const selected = (r.width === d.current_resolution.width && 
                        r.height === d.current_resolution.height && 
                        r.refresh_rate === d.current_resolution.refresh_rate) ? "selected" : "";
      return `<option value="${r.width}x${r.height}@${r.refresh_rate}" ${selected}>${r.width}x${r.height} @ ${r.refresh_rate}Hz</option>`;
    }).join("");

    // Build DVC elements (only if supported by NVAPI)
    const hasDvc = d.max_vibrance > 0;
    const dvcPercent = hasDvc ? Math.round(((d.current_vibrance - d.min_vibrance) / (d.max_vibrance - d.min_vibrance)) * 100) : 50;

    let dvcHtml = "";
    if (hasDvc) {
      dvcHtml = `
        <div class="control-group">
          <div class="control-label">
            <span>Digital Vibrance</span>
            <span class="control-val" id="dvc-val-${d.id}">${dvcPercent}%</span>
          </div>
          <div class="slider-container">
            <input type="range" class="range-slider dvc-slider" 
                   data-display-id="${d.id}" min="0" max="100" value="${dvcPercent}" />
          </div>
        </div>
      `;
    } else {
      dvcHtml = `
        <div class="control-group">
          <div class="control-label">
            <span>Digital Vibrance</span>
            <span class="control-val text-muted">Not Supported (NVIDIA Only)</span>
          </div>
        </div>
      `;
    }

    card.innerHTML = `
      <div class="display-card-header">
        <div>
          <div class="display-name">${d.name}</div>
          <div class="display-id">${d.id}</div>
        </div>
        ${primaryTag}
      </div>

      <div class="control-group">
        <div class="control-label">Resolution & Refresh Rate</div>
        <select class="res-select" data-display-id="${d.id}">
          ${resOptions}
        </select>
      </div>

      ${dvcHtml}

      <div class="control-group">
        <div class="control-label">
          <span>Gamma Boost</span>
          <span class="control-val" id="gamma-val-${d.id}">1.0</span>
        </div>
        <div class="slider-container">
          <input type="range" class="range-slider gamma-slider" 
                 data-display-id="${d.id}" min="50" max="300" value="100" />
        </div>
      </div>
    `;

    displaysContainer.appendChild(card);
  });

  // Bind change listeners
  document.querySelectorAll(".res-select").forEach(select => {
    select.addEventListener("change", async (e) => {
      const displayId = e.target.getAttribute("data-display-id");
      const [res, rate] = e.target.value.split("@");
      const [w, h] = res.split("x");
      await invoke("apply_resolution", { displayId, width: parseInt(w), height: parseInt(h), refreshRate: parseInt(rate) });
      showToast("Resolution changed.", "success");
    });
  });

  document.querySelectorAll(".dvc-slider").forEach(slider => {
    slider.addEventListener("input", async (e) => {
      const displayId = e.target.getAttribute("data-display-id");
      const val = parseInt(e.target.value);
      document.getElementById(`dvc-val-${displayId}`).textContent = `${val}%`;
      await invoke("apply_vibrance", { displayId, vibrancePercent: val });
    });
  });

  document.querySelectorAll(".gamma-slider").forEach(slider => {
    slider.addEventListener("input", async (e) => {
      const displayId = e.target.getAttribute("data-display-id");
      const val = parseInt(e.target.value) / 100.0;
      document.getElementById(`gamma-val-${displayId}`).textContent = val.toFixed(2);
      await invoke("apply_gamma", { displayId, gamma: val });
    });
  });
}

// Load and refresh application profiles
async function refreshProfilesList() {
  try {
    profiles = await invoke("get_profiles");
    renderProfilesList();
  } catch (err) {
    console.error("Failed to load profiles:", err);
  }
}

function renderProfilesList() {
  profilesListContainer.innerHTML = "";
  
  if (profiles.length === 0) {
    profilesListContainer.innerHTML = "<p class='text-muted' style='text-align: center; padding: 12px;'>No profiles.</p>";
    return;
  }

  profiles.forEach((p, idx) => {
    const item = document.createElement("div");
    item.className = `profile-item ${activeProfileId === idx ? "selected" : ""}`;
    
    // Default fallback tag
    const defaultTag = p.is_default ? "<span class='badge' style='background-color: var(--pastel-yellow-bg); color: var(--pastel-yellow-txt); margin-left: 6px; font-size: 8px; padding: 2px 5px; text-transform: uppercase;'>Default</span>" : "";
    
    // Hotkey representation
    const hotkeyInfo = p.hotkey ? `<span style="font-family: monospace; font-size: 9px; color: var(--text-muted); float: right; padding: 2px 4px; border: 1px solid var(--border-primary); border-radius: var(--radius-sm); background: var(--bg-sidebar);">${p.hotkey}</span>` : "";

    item.innerHTML = `
      <div style="display: flex; justify-content: space-between; align-items: center; width: 100%;">
        <div style="flex-grow: 1; overflow: hidden; text-overflow: ellipsis;">
          <div style="display: flex; align-items: center; gap: 4px;">
            <span style="font-weight: 600;">${p.friendly_name}</span>
            ${defaultTag}
          </div>
          <div class="profile-item-exe" style="display: flex; align-items: center; justify-content: space-between; margin-top: 3px;">
            <span>${p.executable_name || "Manual Only"}</span>
            ${hotkeyInfo}
          </div>
        </div>
        <button class="btn-profile-apply-mini" data-idx="${idx}" title="Apply settings now" style="margin-left: 10px; flex-shrink: 0;">
          <svg style="width:10px;height:10px;fill:currentColor" viewBox="0 0 24 24">
            <polygon points="5 3 19 12 5 21 5 3"/>
          </svg>
        </button>
      </div>
    `;

    item.addEventListener("click", (e) => {
      if (e.target.closest(".btn-profile-apply-mini")) {
        e.stopPropagation();
        applyProfileManually(idx);
      } else {
        selectProfile(idx);
      }
    });

    profilesListContainer.appendChild(item);
  });
}

// Apply settings for a specific profile manually
async function applyProfileManually(index) {
  const profile = profiles[index];
  showToast(`Enforcing "${profile.friendly_name}" settings...`, "info");

  let success = true;
  for (const [displayId, settings] of Object.entries(profile.settings)) {
    try {
      const ok = await invoke("trigger_manual_apply", {
        displayId,
        settings: {
          resolution: settings.resolution,
          vibrance: settings.vibrance,
          gamma: settings.gamma
        }
      });
      success = success && ok;
    } catch (err) {
      console.error(`Failed to manually apply setting on screen ${displayId}:`, err);
      success = false;
    }
  }

  if (success) {
    showToast(`Profile "${profile.friendly_name}" applied successfully.`, "success");
    updateActiveProfileBanner(profile.friendly_name);
  } else {
    showToast("Failed to apply profile configurations.", "error");
  }
}

// Handle profile selection / editing
function selectProfile(index) {
  activeProfileId = index;
  renderProfilesList();
  
  const profile = profiles[index];
  
  // Fill inputs
  document.getElementById("profile-friendly-name").value = profile.friendly_name;
  document.getElementById("profile-exe-name").value = profile.executable_name || "";
  document.getElementById("profile-enabled").checked = profile.is_enabled;
  document.getElementById("profile-is-default").checked = !!profile.is_default;
  document.getElementById("profile-hotkey").value = profile.hotkey || "";

  // Render Display tabs in editor
  renderProfileDisplayTabs(profile);

  profileEditorPlaceholder.style.display = "none";
  profileEditor.style.display = "flex";
}

function renderProfileDisplayTabs(profile) {
  displayTabsHeader.innerHTML = "";
  displayTabsContent.innerHTML = "";

  if (displays.length === 0) {
    displayTabsContent.innerHTML = "<p class='text-muted'>Refresh displays to configure settings.</p>";
    return;
  }

  if (!activeProfileTabDisplayId || !displays.some(d => d.id === activeProfileTabDisplayId)) {
    activeProfileTabDisplayId = displays[0].id;
  }

  displays.forEach((d, idx) => {
    // Header tab button
    const btn = document.createElement("button");
    btn.type = "button";
    btn.className = `tab-btn ${d.id === activeProfileTabDisplayId ? "active" : ""}`;
    btn.textContent = d.name;
    btn.addEventListener("click", () => {
      activeProfileTabDisplayId = d.id;
      renderProfileDisplayTabs(profile);
    });
    displayTabsHeader.appendChild(btn);

    const setting = profile.settings[d.id] || { resolution: null, vibrance: null, gamma: null };
    const hasOverride = profile.settings[d.id] !== undefined;

    const content = document.createElement("div");
    content.className = `display-settings-tab ${d.id === activeProfileTabDisplayId ? "active" : ""}`;

    let resOptions = `<option value="native" ${!setting.resolution ? "selected" : ""}>Keep Native</option>`;
    resOptions += d.supported_resolutions.map(r => {
      const selected = (setting.resolution && 
                        r.width === setting.resolution.width && 
                        r.height === setting.resolution.height && 
                        r.refresh_rate === setting.resolution.refresh_rate) ? "selected" : "";
      return `<option value="${r.width}x${r.height}@${r.refresh_rate}" ${selected}>${r.width}x${r.height} @ ${r.refresh_rate}Hz</option>`;
    }).join("");

    const hasDvc = d.max_vibrance > 0;
    const currentVibrance = setting.vibrance !== null ? setting.vibrance : 50;
    const currentGamma = setting.gamma !== null ? setting.gamma : 1.0;

    content.innerHTML = `
      <div class="form-group checkbox-group">
        <input type="checkbox" id="override-display-${idx}" ${hasOverride ? "checked" : ""} />
        <label for="override-display-${idx}">Override color/resolution settings for this screen</label>
      </div>

      <div class="override-controls-${idx}" style="display: ${hasOverride ? 'flex' : 'none'}; flex-direction: column; gap: 20px;">
        <div class="form-group">
          <label>Target Resolution</label>
          <select id="profile-res-${idx}">
            ${resOptions}
          </select>
        </div>

        ${hasDvc ? `
          <div class="form-group">
            <div class="control-label">
              <label>Digital Vibrance Override</label>
              <span class="control-val" id="profile-dvc-val-${idx}">${currentVibrance}%</span>
            </div>
            <div class="slider-container">
              <input type="range" class="range-slider profile-dvc-slider" data-idx="${idx}"
                     id="profile-dvc-${idx}" min="0" max="100" value="${currentVibrance}" />
            </div>
          </div>
        ` : ''}

        <div class="form-group">
          <div class="control-label">
            <label>Gamma Boost Override</label>
            <span class="control-val" id="profile-gamma-val-${idx}">${currentGamma.toFixed(2)}</span>
          </div>
          <div class="slider-container">
            <input type="range" class="range-slider profile-gamma-slider" data-idx="${idx}"
                   id="profile-gamma-${idx}" min="50" max="300" value="${Math.round(currentGamma * 100)}" />
          </div>
        </div>
      </div>
    `;

    displayTabsContent.appendChild(content);

    const chkOverride = content.querySelector(`#override-display-${idx}`);
    const divControls = content.querySelector(`.override-controls-${idx}`);
    
    chkOverride.addEventListener("change", (e) => {
      divControls.style.display = e.target.checked ? "flex" : "none";
      if (e.target.checked) {
        profile.settings[d.id] = { resolution: null, vibrance: hasDvc ? 50 : null, gamma: 1.0 };
      } else {
        delete profile.settings[d.id];
      }
    });

    if (hasDvc) {
      content.querySelector(`#profile-dvc-${idx}`).addEventListener("input", (e) => {
        document.getElementById(`profile-dvc-val-${idx}`).textContent = `${e.target.value}%`;
      });
    }

    content.querySelector(`#profile-gamma-${idx}`).addEventListener("input", (e) => {
      const val = parseInt(e.target.value) / 100.0;
      document.getElementById(`profile-gamma-val-${idx}`).textContent = val.toFixed(2);
    });
  });
}

// Global states loaders
async function checkDaemonStatus() {
  try {
    const isDaemonActive = await invoke("is_daemon_active");
    toggleDaemon.checked = isDaemonActive;
    updateDaemonStatusUI(isDaemonActive);
  } catch (err) {
    console.error("Failed to check daemon status:", err);
  }
}

function updateDaemonStatusUI(active) {
  if (active) {
    daemonDot.classList.add("active");
    daemonStatusText.textContent = "Daemon Active";
  } else {
    daemonDot.classList.remove("active");
    daemonStatusText.textContent = "Daemon Paused";
  }
}

// Active Profile display state
async function checkActiveProfile() {
  try {
    const active = await invoke("get_active_profile");
    updateActiveProfileBanner(active);
  } catch (err) {
    console.error("Failed to get active profile:", err);
  }
}

function updateActiveProfileBanner(name) {
  if (name) {
    activeProfileBanner.style.display = "block";
    activeProfileName.textContent = name;
  } else {
    activeProfileBanner.style.display = "none";
  }
}

// Load global hotkey settings
async function loadGlobalSettings() {
  try {
    const settings = await invoke("get_global_settings");
    document.getElementById("global-reset-hotkey").value = settings.reset_hotkey || "";
    document.getElementById("global-daemon-hotkey").value = settings.daemon_hotkey || "";
  } catch (err) {
    console.error("Failed to load global hotkey settings:", err);
  }
}

// DOM Setup
window.addEventListener("DOMContentLoaded", async () => {
  // Elements binding
  displaysContainer = document.getElementById("displays-container");
  profilesListContainer = document.getElementById("profiles-list");
  profileEditor = document.getElementById("profile-editor");
  profileEditorPlaceholder = document.getElementById("profile-editor-placeholder");
  profileForm = document.getElementById("profile-form");
  btnAddProfile = document.getElementById("btn-add-profile");
  btnDeleteProfile = document.getElementById("btn-delete-profile");
  btnCancelEdit = document.getElementById("btn-cancel-edit");
  btnApplyManual = document.getElementById("btn-apply-profile-manual");
  btnRefreshDisplays = document.getElementById("btn-refresh-displays");
  btnGlobalReset = document.getElementById("btn-global-reset");
  toggleDaemon = document.getElementById("toggle-daemon");
  daemonDot = document.getElementById("daemon-dot");
  daemonStatusText = document.getElementById("daemon-status-text");
  activeProfileBanner = document.getElementById("active-profile-banner");
  activeProfileName = document.getElementById("active-profile-name");
  displayTabsHeader = document.getElementById("display-tabs-header");
  displayTabsContent = document.getElementById("display-tabs-content");

  // Initial navigations
  initNavigation();
  await refreshDisplays();
  await refreshProfilesList();
  await checkDaemonStatus();
  await checkActiveProfile();

  // Listen to profile switch changes from Rust background thread
  await listen("profile-changed", (event) => {
    updateActiveProfileBanner(event.payload);
  });

  await listen("daemon-changed", (event) => {
    const active = event.payload;
    toggleDaemon.checked = active;
    updateDaemonStatusUI(active);
    showToast(active ? "Daemon auto-switching enabled." : "Daemon auto-switching paused.", "info");
  });

  await listen("displays-reset", async () => {
    await refreshDisplays();
    updateActiveProfileBanner(null);
    showToast("Monitors reset to default values.", "info");
  });

  // Action listeners
  btnRefreshDisplays.addEventListener("click", async () => {
    await refreshDisplays();
    showToast("Monitors list updated.", "success");
  });
  
  toggleDaemon.addEventListener("change", async (e) => {
    const active = e.target.checked;
    await invoke("set_daemon_active", { active });
    updateDaemonStatusUI(active);
    showToast(active ? "Auto-switching active." : "Auto-switching paused.", "info");
  });

  btnGlobalReset.addEventListener("click", async () => {
    if (await customConfirm("Reset Displays", "Revert all monitors back to standard Windows calibration?")) {
      await invoke("trigger_reset");
      await refreshDisplays();
      showToast("Displays reset successfully.", "success");
    }
  });

  btnApplyManual.addEventListener("click", async () => {
    if (activeProfileId !== null) {
      await applyProfileManually(activeProfileId);
    }
  });

  btnAddProfile.addEventListener("click", () => {
    const newProfile = {
      friendly_name: "New Profile",
      executable_name: "game.exe",
      is_enabled: true,
      settings: {},
      hotkey: null,
      is_default: false
    };
    profiles.push(newProfile);
    activeProfileId = profiles.length - 1;
    renderProfilesList();
    selectProfile(activeProfileId);
    showToast("Draft profile created.", "info");
  });

  btnCancelEdit.addEventListener("click", () => {
    activeProfileId = null;
    renderProfilesList();
    profileEditor.style.display = "none";
    profileEditorPlaceholder.style.display = "flex";
  });

  btnDeleteProfile.addEventListener("click", async () => {
    if (activeProfileId !== null) {
      if (await customConfirm("Delete Profile", `Are you sure you want to delete this profile?`)) {
        profiles.splice(activeProfileId, 1);
        await invoke("save_profiles", { profiles });
        activeProfileId = null;
        await refreshProfilesList();
        profileEditor.style.display = "none";
        profileEditorPlaceholder.style.display = "flex";
        showToast("Profile deleted successfully.", "success");
      }
    }
  });

  // Save global settings
  document.getElementById("btn-save-global-settings").addEventListener("click", async () => {
    const resetHotkey = document.getElementById("global-reset-hotkey").value.trim();
    const daemonHotkey = document.getElementById("global-daemon-hotkey").value.trim();

    try {
      await invoke("save_global_settings", {
        settings: { reset_hotkey: resetHotkey, daemon_hotkey: daemonHotkey }
      });
      showToast("Global shortcut bindings saved successfully.", "success");
    } catch (err) {
      console.error(err);
      showToast("Failed to save global shortcut bindings.", "error");
    }
  });

  profileForm.addEventListener("submit", async (e) => {
    e.preventDefault();
    if (activeProfileId === null) return;

    const friendlyName = document.getElementById("profile-friendly-name").value;
    const exeName = document.getElementById("profile-exe-name").value;
    const isEnabled = document.getElementById("profile-enabled").checked;
    const isDefault = document.getElementById("profile-is-default").checked;
    const hotkeyStr = document.getElementById("profile-hotkey").value.trim();

    const activeProfile = profiles[activeProfileId];
    activeProfile.friendly_name = friendlyName;
    activeProfile.executable_name = exeName;
    activeProfile.is_enabled = isEnabled;
    activeProfile.is_default = isDefault;
    activeProfile.hotkey = hotkeyStr || null;

    if (isDefault) {
      // Radio button behavior for defaults
      profiles.forEach((p, idx) => {
        if (idx !== activeProfileId) {
          p.is_default = false;
        }
      });
    }

    displays.forEach((d, idx) => {
      const chkOverride = document.getElementById(`override-display-${idx}`);
      if (chkOverride && chkOverride.checked) {
        const settings = {};
        
        // Resolution
        const resVal = document.getElementById(`profile-res-${idx}`).value;
        if (resVal !== "native") {
          const [res, rate] = resVal.split("@");
          const [w, h] = res.split("x");
          settings.resolution = {
            width: parseInt(w),
            height: parseInt(h),
            refresh_rate: parseInt(rate)
          };
        } else {
          settings.resolution = null;
        }

        // Vibrance
        const sliderDvc = document.getElementById(`profile-dvc-${idx}`);
        if (sliderDvc) {
          settings.vibrance = parseInt(sliderDvc.value);
        } else {
          settings.vibrance = null;
        }

        // Gamma
        const sliderGamma = document.getElementById(`profile-gamma-${idx}`);
        settings.gamma = parseInt(sliderGamma.value) / 100.0;

        activeProfile.settings[d.id] = settings;
      } else {
        delete activeProfile.settings[d.id];
      }
    });

    // Save to file
    await invoke("save_profiles", { profiles });
    showToast("Profile saved successfully.", "success");
    await refreshProfilesList();
  });
});
