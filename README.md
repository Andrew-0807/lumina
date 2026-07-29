<div align="center">

  <img src="src-tauri/icons/128x128.png" width="112" height="112" alt="Lumina logo" />

  # _Lumina_
  
  **A Premium, Minimalist Display & Color Controller for Windows**
  
  _An elegant, open-source utility for managing monitor resolutions and digital vibrance via hotkeys and manual presets._

  ---
  
  [![Tauri Version](https://img.shields.io/badge/tauri-v2.0-blueviolet?style=flat-square)](https://tauri.app)
  [![Rust Backend](https://img.shields.io/badge/rust-backend-orange?style=flat-square)](https://www.rust-lang.org)
  [![License](https://img.shields.io/badge/license-MIT-lightgrey?style=flat-square)](LICENSE)
  [![Platform](https://img.shields.io/badge/platform-windows-blue?style=flat-square)](https://microsoft.com/windows)

</div>

## 🎬 Overview

**Lumina** is a lightweight, high-performance display control utility for Windows. It consolidates display resolution switching and NVIDIA Digital Vibrance configuration into a clean, modern user interface driven by custom hotkeys and manual profile triggers.

Lumina operates exclusively through official Windows GDI display APIs (`ChangeDisplaySettingsExW`) and official NVIDIA driver functions (`nvapi64.dll`). It runs purely as an on-demand desktop display management tool with **no background process scanning**.

---

## 🎨 Design System: Warm Monochrome

Lumina is designed around a **Premium Utilitarian Minimalist Editorial** theme:
*   **Warm Charcoal Canvas:** Deep dark theme utilizing soft charcoal tones (`#121214`) instead of harsh absolute black.
*   **Crisp Bento Grid:** Responsive flat bento boxes featuring subtle `#2a2a2f` borders.
*   **Physical Micro-UIs:** Custom keyboard shortcuts represented inside `<kbd>` containers.
*   **Custom Micro-Animations:** Responsive custom controls, Webkit scrollbars, and toast notifications.

---

## ✨ Features

*   **⚡ Native Display Controls:** Interacts directly with Windows display settings (`ChangeDisplaySettingsExW`) and NVIDIA NVAPI (`NvAPI_SetDVCLevel`) for smooth hardware-level resolution and color vibrance changes.
*   **🎹 Dynamic Global Hotkeys:** Bind hotkeys dynamically to switch profiles or reset display settings at any time.
*   **🔀 Toggle Profile Bindings:** Easily toggle active profiles on and off with custom keyboard shortcuts.
*   **🖱️ Manual Presets:** Apply display configurations manually on demand via the sidebar or profile editor.
*   **🔔 Elegant Toast Notifications:** Built-in animated status toasts and confirmation overlays.
*   **📥 Minimize to Tray:** Hides seamlessly into the Windows system tray on close.

---

## 🎹 Default Shortcut Bindings

Global shortcuts can be customized or triggered at any time:
*   **Reset Displays:** `<kbd>Ctrl</kbd> + <kbd>Alt</kbd> + <kbd>R</kbd>` (Reverts all connected screens to standard Windows defaults).
*   **Custom Profiles:** Bind any display profile to shortcuts like `<kbd>Ctrl</kbd> + <kbd>Alt</kbd> + <kbd>1</kbd>` to switch on demand.

---

## 🛠️ Installation & Building

### Download

Grab the latest installer from the releases page:

> **➡️ [Download the latest release](https://github.com/Andrew-0807/lumina/releases/latest)** (`.msi`)

### Development Setup

To build and run locally:

1.  Clone the repository:
    ```bash
    git clone https://github.com/Andrew-0807/lumina.git
    cd lumina
    ```
2.  Install dependencies:
    ```bash
    npm install
    ```
3.  Run in dev mode:
    ```bash
    npm run tauri dev
    ```
4.  Build for production:
    ```bash
    npm run tauri build
    ```

---

## 🔒 Security & Safe Operation

* **No Background Process Polling:** Lumina contains zero background process scanners or process handle queries (`OpenProcess`). It runs purely as an on-demand, hotkey-driven utility.
* **Standard APIs:** All display modifications use standard Windows GDI and official NVIDIA driver DLL interfaces (`nvapi64.dll`).

---

## 📄 License

Lumina is released under the [MIT License](LICENSE).
