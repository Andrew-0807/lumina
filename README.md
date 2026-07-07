<div align="center">

  <img src="src-tauri/icons/128x128.png" width="112" height="112" alt="Lumina logo" />

  # _Lumina_
  
  **A Premium, Minimalist Display & Color Controller for Windows**
  
  _An elegant, all-in-one alternative to VibranceGUI, Quick Res Changer, and Gamma Control._

  ---
  
  [![Tauri Version](https://img.shields.io/badge/tauri-v2.0-blueviolet?style=flat-square)](https://tauri.app)
  [![Rust Backend](https://img.shields.io/badge/rust-backend-orange?style=flat-square)](https://www.rust-lang.org)
  [![License](https://img.shields.io/badge/license-MIT-lightgrey?style=flat-square)](LICENSE)
  [![Platform](https://img.shields.io/badge/platform-windows-blue?style=flat-square)](https://microsoft.com/windows)

</div>

## 🎬 Showcase

<div align="center">



https://github.com/user-attachments/assets/dd571940-b1bf-4bd6-92c6-e9e17ab976db





</div>

---

## Overview

**Lumina** is a lightweight, high-performance display control utility designed for power users and gamers. It consolidates multiple display configuration tools into a single native Tauri app, allowing you to quickly change resolutions, digital vibrance levels, and gamma ramps. 

Lumina works at the driver level (GDI and NVAPI) without injecting code or modifying game memory, making it **100% safe for anti-cheat systems** in competitive titles like *Rust*, *CS2*, and *Valorant*.

---

## 🎨 Design System: Warm Monochrome

Lumina is designed around a **Premium Utilitarian Minimalist Editorial** theme:
*   **Warm Charcoal Canvas:** Deep dark theme utilizing soft charcoal bones (`#121214`) instead of cold absolute black.
*   **Crisp Bento Grid:** Responsive flat bento boxes featuring subtle `#2a2a2f` borders.
*   **Physical Micro-UIs:** Custom keyboard shortcuts represented inside `<kbd>` containers.
*   **Custom Micro-Animations:** Responsive custom checkboxes, custom Webkit scrollbars, and toast notifications.

---

## ✨ Features

*   **⚡ Native Driver Controls:** Interacts directly with Windows GDI display settings (`ChangeDisplaySettingsExW`, `SetDeviceGammaRamp`) and NVIDIA NVAPI (`NvAPI_SetDVCLevel`) for smooth hardware-level changes.
*   **🔄 Focus Daemon (Auto-Switching):** Automatically monitors active windows and applies target profiles (e.g. boosting digital vibrance and gamma when *RustClient.exe* is active) and reverts to your defaults when the process loses focus.
*   **🎹 Dynamic Global Hotkeys:** Set global hotkeys dynamically. Hit your shortcut anywhere to trigger profiles.
*   **🔀 Toggle Profile Bindings:** Pressing an active profile's shortcut toggles it off, falling back to a designated **`DEFAULT`** profile (e.g. Day mode config) or standard Windows defaults.
*   **🖱️ Manual Controls:** Apply profiles manually using a quick-apply mini play button on the sidebar list or directly from the editor without turning on background scanner checks.
*   **🔔 Elegant Toast Notifications:** Built-in animated status toasts and confirmation overlays.
*   **📥 Minimize to Tray:** Hides seamlessly into the Windows system tray on close.

---

## 🎹 Shortcut Bindings

Global shortcuts are bound directly via native Win32 messages:
*   **Emergency Reset:** `<kbd>Ctrl</kbd> + <kbd>Alt</kbd> + <kbd>R</kbd>` (Reverts all connected screens back to standard Windows defaults).
*   **Toggle Daemon:** `<kbd>Ctrl</kbd> + <kbd>Alt</kbd> + <kbd>D</kbd>` (Pauses or activates background focus scanner auto-switching).
*   **Custom Profiles:** Bind any profile to keys like `<kbd>Ctrl</kbd> + <kbd>Alt</kbd> + <kbd>1</kbd>` or `<kbd>Ctrl</kbd> + <kbd>Alt</kbd> + <kbd>P</kbd>` to toggle them on demand.

---

## 🛠️ Installation & Building

### Download

Grab the latest signed installer from the releases page:

> **➡️ [Download the latest release](https://github.com/Andrew-0807/lumina/releases/latest)** (`.msi`)

Once installed, **Lumina updates itself** — it checks GitHub for new releases on launch and installs them automatically, so you only download once.

### Development Setup
To run and develop locally:

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

## 🔒 Security & Safe Play

Lumina utilizes Windows system APIs and the official NVIDIA Display Driver wrapper DLL (`nvapi64.dll`). It **does not read or write game memory**, hook into game overlay threads, or modify DirectX/Vulkan frames. This makes it completely invisible to anti-cheat hooks (EAC, BattlEye, Vanguard) and safe to run alongside any competitive game.
