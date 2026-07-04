#![allow(non_camel_case_types)]

use std::ffi::{c_char, c_void, CString};
use std::ptr;
use std::sync::Mutex;
use std::path::Path;
use serde::{Serialize, Deserialize};

// Win32 Imports from windows-sys
use windows_sys::Win32::Foundation::CloseHandle;
use windows_sys::Win32::Graphics::Gdi::{
    CreateDCW, DeleteDC, EnumDisplayDevicesW, EnumDisplaySettingsW,
    DISPLAY_DEVICEW, DEVMODEW,
    ENUM_CURRENT_SETTINGS, DM_PELSWIDTH, DM_PELSHEIGHT, DM_DISPLAYFREQUENCY,
    ChangeDisplaySettingsExW, CDS_UPDATEREGISTRY
};
use windows_sys::Win32::UI::ColorSystem::SetDeviceGammaRamp;
use windows_sys::Win32::System::LibraryLoader::{GetProcAddress, LoadLibraryA};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    GetForegroundWindow, GetWindowThreadProcessId
};
use windows_sys::Win32::System::Threading::{
    OpenProcess, QueryFullProcessImageNameW, PROCESS_QUERY_LIMITED_INFORMATION
};

// NVAPI Struct definitions
#[repr(C)]
#[derive(Debug, Clone, Copy)]
struct NvDVCInfo {
    pub version: u32,
    pub current_level: i32,
    pub min_level: i32,
    pub max_level: i32,
}

// Function types for NVAPI
type NvAPI_InitializeFn = unsafe extern "C" fn() -> i32;
type NvAPI_GetAssociatedNvidiaDisplayHandleFn = unsafe extern "C" fn(
    display_name: *const c_char,
    p_handle: *mut *mut c_void
) -> i32;
type NvAPI_GetDVCInfoFn = unsafe extern "C" fn(
    handle: *mut c_void,
    output_id: u32,
    p_dvc_info: *mut NvDVCInfo
) -> i32;
type NvAPI_SetDVCLevelFn = unsafe extern "C" fn(
    handle: *mut c_void,
    output_id: u32,
    level: i32
) -> i32;

// Static container for NVAPI function pointers
struct NvApiState {
    initialized: bool,
    enum_display: Option<unsafe extern "C" fn(u32, *mut *mut c_void) -> i32>,
    get_associated_handle: Option<NvAPI_GetAssociatedNvidiaDisplayHandleFn>,
    get_dvc_info: Option<NvAPI_GetDVCInfoFn>,
    set_dvc_level: Option<NvAPI_SetDVCLevelFn>,
}

use std::sync::OnceLock;

// ponytail: using standard library std::sync::OnceLock instead of lazy_static crate to minimize dependencies.
static NVAPI: OnceLock<Mutex<NvApiState>> = OnceLock::new();

fn get_nvapi() -> &'static Mutex<NvApiState> {
    NVAPI.get_or_init(|| Mutex::new(load_nvapi()))
}

fn load_nvapi() -> NvApiState {
    let mut state = NvApiState {
        initialized: false,
        enum_display: None,
        get_associated_handle: None,
        get_dvc_info: None,
        set_dvc_level: None,
    };

    unsafe {
        let h_module = LoadLibraryA(b"nvapi64.dll\0".as_ptr());
        if h_module == 0 {
            return state;
        }

        let query_interface_addr = GetProcAddress(h_module, b"nvapi_QueryInterface\0".as_ptr());
        if query_interface_addr.is_none() {
            return state;
        }

        let query_interface: unsafe extern "C" fn(u32) -> *const c_void =
            std::mem::transmute(query_interface_addr.unwrap());

        // Helper to query function pointers
        let get_func = |id: u32| -> Option<*const c_void> {
            let ptr = query_interface(id);
            if ptr.is_null() {
                None
            } else {
                Some(ptr)
            }
        };

        // Initialize NVAPI
        if let Some(init_ptr) = get_func(0x0150E828) {
            let init_fn: NvAPI_InitializeFn = std::mem::transmute(init_ptr);
            if init_fn() == 0 {
                state.initialized = true;
                
                if let Some(enum_ptr) = get_func(0x9ABDD40D) {
                    state.enum_display = Some(std::mem::transmute(enum_ptr));
                }
                if let Some(get_handle_ptr) = get_func(0x35C29134) {
                    state.get_associated_handle = Some(std::mem::transmute(get_handle_ptr));
                }
                if let Some(get_dvc_ptr) = get_func(0x4085DE45) {
                    state.get_dvc_info = Some(std::mem::transmute(get_dvc_ptr));
                }
                if let Some(set_dvc_ptr) = get_func(0x172409B4) {
                    state.set_dvc_level = Some(std::mem::transmute(set_dvc_ptr));
                }
            }
        }
    }

    state
}

// Data models
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct Resolution {
    pub width: u32,
    pub height: u32,
    pub refresh_rate: u32,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct DisplayInfo {
    pub id: String,                  // e.g., "\\.\DISPLAY1"
    pub name: String,                // e.g., "ASUS VG248"
    pub is_primary: bool,
    pub current_resolution: Resolution,
    pub supported_resolutions: Vec<Resolution>,
    pub min_vibrance: i32,
    pub max_vibrance: i32,
    pub current_vibrance: i32,
}

// Windows UTF-16 string conversion helper
fn u16_to_string(slice: &[u16]) -> String {
    let len = slice.iter().position(|&x| x == 0).unwrap_or(slice.len());
    String::from_utf16_lossy(&slice[..len])
}

fn string_to_u16_vec(s: &str) -> Vec<u16> {
    let mut v: Vec<u16> = s.encode_utf16().collect();
    v.push(0);
    v
}

pub fn get_connected_displays() -> Vec<DisplayInfo> {
    let mut displays = Vec::new();
    let nv = get_nvapi().lock().unwrap();

    unsafe {
        let mut dev_idx = 0;
        loop {
            let mut device: DISPLAY_DEVICEW = std::mem::zeroed();
            device.cb = std::mem::size_of::<DISPLAY_DEVICEW>() as u32;

            let res = EnumDisplayDevicesW(ptr::null(), dev_idx, &mut device, 0);
            if res == 0 {
                break;
            }

            // State flags check: DISPLAY_DEVICE_ACTIVE = 0x00000001
            if (device.StateFlags & 0x00000001) != 0 {
                let device_name = u16_to_string(&device.DeviceName);
                let friendly_name = u16_to_string(&device.DeviceString);
                let is_primary = (device.StateFlags & 0x00000004) != 0; // DISPLAY_DEVICE_PRIMARY_DEVICE = 0x00000004

                // Get Current Resolution
                let mut current_res = Resolution { width: 0, height: 0, refresh_rate: 0 };
                let mut dev_mode: DEVMODEW = std::mem::zeroed();
                dev_mode.dmSize = std::mem::size_of::<DEVMODEW>() as u16;
                if EnumDisplaySettingsW(device.DeviceName.as_ptr(), ENUM_CURRENT_SETTINGS, &mut dev_mode) != 0 {
                    current_res = Resolution {
                        width: dev_mode.dmPelsWidth,
                        height: dev_mode.dmPelsHeight,
                        refresh_rate: dev_mode.dmDisplayFrequency,
                    };
                }

                // Get Supported Resolutions
                let mut supported_resolutions = Vec::new();
                let mut mode_idx = 0;
                loop {
                    let mut mode: DEVMODEW = std::mem::zeroed();
                    mode.dmSize = std::mem::size_of::<DEVMODEW>() as u16;
                    if EnumDisplaySettingsW(device.DeviceName.as_ptr(), mode_idx, &mut mode) == 0 {
                        break;
                    }
                    let res_item = Resolution {
                        width: mode.dmPelsWidth,
                        height: mode.dmPelsHeight,
                        refresh_rate: mode.dmDisplayFrequency,
                    };
                    if !supported_resolutions.contains(&res_item) {
                        supported_resolutions.push(res_item);
                    }
                    mode_idx += 1;
                }

                // Sort resolutions in descending order of size/refresh rate
                supported_resolutions.sort_by(|a, b| {
                    b.width.cmp(&a.width)
                        .then(b.height.cmp(&a.height))
                        .then(b.refresh_rate.cmp(&a.refresh_rate))
                });

                // Get DVC Vibrance info
                let mut min_vibrance = 0;
                let mut max_vibrance = 0;
                let mut current_vibrance = 0;

                if nv.initialized {
                    if let Some(get_handle) = nv.get_associated_handle {
                        if let Some(get_dvc) = nv.get_dvc_info {
                            let mut dvc_handle: *mut c_void = ptr::null_mut();
                            let ascii_name = CString::new(device_name.clone()).unwrap();
                            let nv_res = get_handle(ascii_name.as_ptr(), &mut dvc_handle);
                            if nv_res == 0 && !dvc_handle.is_null() {
                                let mut dvc_info = NvDVCInfo {
                                    version: (std::mem::size_of::<NvDVCInfo>() as u32) | 0x10000,
                                    current_level: 0,
                                    min_level: 0,
                                    max_level: 0,
                                };
                                let dvc_res = get_dvc(dvc_handle, 0, &mut dvc_info);
                                if dvc_res == 0 {
                                    current_vibrance = dvc_info.current_level;
                                    min_vibrance = dvc_info.min_level;
                                    max_vibrance = dvc_info.max_level;
                                }
                            }
                        }
                    }
                }

                displays.push(DisplayInfo {
                    id: device_name,
                    name: friendly_name,
                    is_primary,
                    current_resolution: current_res,
                    supported_resolutions,
                    min_vibrance,
                    max_vibrance,
                    current_vibrance,
                });
            }

            dev_idx += 1;
        }
    }

    displays
}

pub fn apply_vibrance(display_id: &str, vibrance_percent: i32) -> bool {
    let nv = get_nvapi().lock().unwrap();
    if !nv.initialized {
        return false;
    }

    unsafe {
        if let Some(get_handle) = nv.get_associated_handle {
            if let Some(get_dvc) = nv.get_dvc_info {
                if let Some(set_dvc) = nv.set_dvc_level {
                    let mut dvc_handle: *mut c_void = ptr::null_mut();
                    let ascii_name = CString::new(display_id).unwrap();
                    let nv_res = get_handle(ascii_name.as_ptr(), &mut dvc_handle);
                    if nv_res == 0 && !dvc_handle.is_null() {
                        let mut dvc_info = NvDVCInfo {
                            version: (std::mem::size_of::<NvDVCInfo>() as u32) | 0x10000,
                            current_level: 0,
                            min_level: 0,
                            max_level: 0,
                        };
                        if get_dvc(dvc_handle, 0, &mut dvc_info) == 0 {
                            // Calculate DVC level based on percentage
                            let target_level = dvc_info.min_level +
                                ((dvc_info.max_level - dvc_info.min_level) as f32 * (vibrance_percent as f32 / 100.0)) as i32;
                            let set_res = set_dvc(dvc_handle, 0, target_level);
                            return set_res == 0;
                        }
                    }
                }
            }
        }
    }
    false
}

pub fn apply_gamma(display_id: &str, gamma: f32) -> bool {
    unsafe {
        let u16_name = string_to_u16_vec(display_id);
        let hdc = CreateDCW(ptr::null(), u16_name.as_ptr(), ptr::null(), ptr::null());
        if hdc == 0 {
            return false;
        }

        let mut ramp = [0u16; 768];
        for i in 0..256 {
            let val = (((i as f32 / 255.0).powf(1.0 / gamma)) * 65535.0 + 0.5) as u32;
            let val = val.max(0).min(65535) as u16;
            ramp[i] = val;       // Red
            ramp[i + 256] = val; // Green
            ramp[i + 512] = val; // Blue
        }

        let res = SetDeviceGammaRamp(hdc, ramp.as_ptr() as *const _);
        DeleteDC(hdc);
        res != 0
    }
}

pub fn apply_resolution(display_id: &str, width: u32, height: u32, refresh_rate: u32) -> bool {
    unsafe {
        let u16_name = string_to_u16_vec(display_id);
        let mut dev_mode: DEVMODEW = std::mem::zeroed();
        dev_mode.dmSize = std::mem::size_of::<DEVMODEW>() as u16;
        dev_mode.dmFields = DM_PELSWIDTH | DM_PELSHEIGHT | DM_DISPLAYFREQUENCY;
        dev_mode.dmPelsWidth = width;
        dev_mode.dmPelsHeight = height;
        dev_mode.dmDisplayFrequency = refresh_rate;

        // Change display settings
        let res = ChangeDisplaySettingsExW(
            u16_name.as_ptr(),
            &dev_mode,
            0,
            CDS_UPDATEREGISTRY,
            ptr::null()
        );
        res == 0
    }
}

pub fn get_foreground_process_name() -> String {
    unsafe {
        let hwnd = GetForegroundWindow();
        if hwnd == 0 {
            return String::new();
        }

        let mut pid = 0u32;
        GetWindowThreadProcessId(hwnd, &mut pid);
        if pid == 0 {
            return String::new();
        }

        let h_process = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
        if h_process == 0 {
            return String::new();
        }

        let mut buf = [0u16; 260];
        let mut size = buf.len() as u32;
        let success = QueryFullProcessImageNameW(h_process, 0, buf.as_mut_ptr(), &mut size);
        CloseHandle(h_process);

        if success != 0 {
            let path_str = u16_to_string(&buf[..size as usize]);
            if let Some(filename) = Path::new(&path_str).file_name() {
                return filename.to_string_lossy().into_owned();
            }
        }

        String::new()
    }
}
