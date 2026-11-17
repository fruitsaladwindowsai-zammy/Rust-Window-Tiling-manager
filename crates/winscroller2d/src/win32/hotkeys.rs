use crate::win32::WM_APP_ACTION;
use std::collections::{HashMap, HashSet};
use std::mem::size_of;
use std::sync::atomic::{AtomicBool, AtomicIsize, AtomicU8, AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock, RwLock};
use std::time::Instant;
use windows_sys::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
    GetAsyncKeyState, INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYEVENTF_KEYUP,
    MOD_ALT as WIN_MOD_ALT, MOD_CONTROL as WIN_MOD_CONTROL, MOD_NOREPEAT as WIN_MOD_NOREPEAT,
    MOD_SHIFT as WIN_MOD_SHIFT, MOD_WIN as WIN_MOD_WIN, RegisterHotKey, SendInput,
    UnregisterHotKey, VK_0, VK_9, VK_A, VK_ADD, VK_BACK, VK_DELETE, VK_DOWN, VK_END, VK_ESCAPE,
    VK_F1, VK_F12, VK_HOME, VK_INSERT, VK_LCONTROL, VK_LEFT, VK_LMENU, VK_LSHIFT, VK_LWIN, VK_NEXT,
    VK_OEM_4, VK_OEM_5, VK_OEM_6, VK_OEM_COMMA, VK_OEM_MINUS, VK_OEM_PERIOD, VK_OEM_PLUS, VK_PRIOR,
    VK_RCONTROL, VK_RETURN, VK_RIGHT, VK_RMENU, VK_RSHIFT, VK_RWIN, VK_SPACE, VK_SUBTRACT, VK_TAB,
    VK_UP, VK_Z,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    CallNextHookEx, HHOOK, KBDLLHOOKSTRUCT, LLKHF_INJECTED, PostMessageW, SetWindowsHookExW,
    UnhookWindowsHookEx, WH_KEYBOARD_LL, WM_KEYDOWN, WM_KEYUP, WM_SYSKEYDOWN, WM_SYSKEYUP,
};
use winscroller2d_core::Action;
use winscroller2d_core::config::KeybindConfig;

const MOD_WIN: u8 = 1 << 0;
const MOD_CTRL: u8 = 1 << 1;
const MOD_SHIFT: u8 = 1 << 2;
const MOD_ALT: u8 = 1 << 3;

const SYNTHETIC_EXTRA: usize = 0x5753_3244;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct Chord {
    mods: u8,
    vk: u32,
}

#[derive(Default)]
struct Keymap {
    generation: u64,
    lookup: HashMap<Chord, usize>,
    actions: Vec<Action>,
}

static KEYMAP: OnceLock<RwLock<Keymap>> = OnceLock::new();
static SUPPRESSED_VKS: OnceLock<Mutex<HashSet<u32>>> = OnceLock::new();
static TARGET_HWND: AtomicIsize = AtomicIsize::new(0);
static PAUSED: AtomicBool = AtomicBool::new(false);
static MOD_STATE: AtomicU8 = AtomicU8::new(0);
static LAST_GENERATION: AtomicU64 = AtomicU64::new(0);
static KEY_CLOCK_EPOCH: OnceLock<Instant> = OnceLock::new();
static LAST_KEY_MS: AtomicU64 = AtomicU64::new(0);

fn now_ms() -> u64 {
    let epoch = KEY_CLOCK_EPOCH.get_or_init(Instant::now);
    epoch.elapsed().as_millis() as u64 + 1
}

/// Milliseconds elapsed since the last non-hotkey keydown observed by the hook,
/// or `u64::MAX` if no keydown has been observed yet.
pub fn ms_since_last_key() -> u64 {
    let last = LAST_KEY_MS.load(Ordering::Relaxed);
    if last == 0 {
        u64::MAX
    } else {
        now_ms().saturating_sub(last)
    }
}

fn keymap_lock() -> &'static RwLock<Keymap> {
    KEYMAP.get_or_init(|| RwLock::new(Keymap::default()))
}

fn suppressed_set() -> &'static Mutex<HashSet<u32>> {
    SUPPRESSED_VKS.get_or_init(|| Mutex::new(HashSet::new()))
}

pub struct HotkeyHook {
    hook: HHOOK,
    hwnd: HWND,
    registered_hotkeys: Vec<i32>,
}

impl HotkeyHook {
    pub fn install(target_hwnd: HWND, bindings: &[KeybindConfig]) -> Result<Self, String> {
        TARGET_HWND.store(target_hwnd as isize, Ordering::SeqCst);
        reload_bindings(bindings);
        let registered_hotkeys = register_system_hotkeys(target_hwnd, bindings);

        let hmod = unsafe { GetModuleHandleW(std::ptr::null()) };
        let hook = unsafe { SetWindowsHookExW(WH_KEYBOARD_LL, Some(keyboard_proc), hmod, 0) };
        if hook.is_null() {
            unregister_system_hotkeys(target_hwnd, &registered_hotkeys);
            return Err("SetWindowsHookExW(WH_KEYBOARD_LL) failed".into());
        }
        Ok(Self {
            hook,
            hwnd: target_hwnd,
            registered_hotkeys,
        })
    }

    pub fn reload(&mut self, bindings: &[KeybindConfig]) {
        unregister_system_hotkeys(self.hwnd, &self.registered_hotkeys);
        reload_bindings(bindings);
        self.registered_hotkeys = register_system_hotkeys(self.hwnd, bindings);
    }
}

impl Drop for HotkeyHook {
    fn drop(&mut self) {
        unregister_system_hotkeys(self.hwnd, &self.registered_hotkeys);
        if !self.hook.is_null() {
            unsafe {
                UnhookWindowsHookEx(self.hook);
            }
        }
        TARGET_HWND.store(0, Ordering::SeqCst);
    }
}

pub fn set_paused(paused: bool) {
    PAUSED.store(paused, Ordering::SeqCst);
}

#[allow(dead_code)]
pub fn current_generation() -> u64 {
    LAST_GENERATION.load(Ordering::SeqCst)
}

pub fn register_system_hotkeys(hwnd: HWND, bindings: &[KeybindConfig]) -> Vec<i32> {
    let mut ids = Vec::new();
    if hwnd.is_null() {
        return ids;
    }
    for (idx, b) in bindings.iter().enumerate() {
        if let Some(chord) = parse_chord(&b.keys) {
            let mut fs = WIN_MOD_NOREPEAT;
            if (chord.mods & MOD_WIN) != 0 {
                fs |= WIN_MOD_WIN;
            }
            if (chord.mods & MOD_CTRL) != 0 {
                fs |= WIN_MOD_CONTROL;
            }
            if (chord.mods & MOD_SHIFT) != 0 {
                fs |= WIN_MOD_SHIFT;
            }
            if (chord.mods & MOD_ALT) != 0 {
                fs |= WIN_MOD_ALT;
            }
            let id = 1000 + idx as i32;
            let ok = unsafe { RegisterHotKey(hwnd, id, fs, chord.vk) };
            if ok != 0 {
                ids.push(id);
            }
        }
    }
    ids
}

pub fn unregister_system_hotkeys(hwnd: HWND, ids: &[i32]) {
    if hwnd.is_null() {
        return;
    }
    for &id in ids {
        unsafe {
            UnregisterHotKey(hwnd, id);
        }
    }
}

pub fn reload_bindings(bindings: &[KeybindConfig]) {
    let mut map = HashMap::new();
    let mut actions = Vec::new();
    for b in bindings {
        if let Some(chord) = parse_chord(&b.keys) {
            let idx = actions.len();
            actions.push(b.action.clone());
            map.insert(chord, idx);
        } else {
            eprintln!(
                "[winscroller2d] ignoring unrecognized keybinding '{}'",
                b.keys
            );
        }
    }
    if let Ok(mut guard) = keymap_lock().write() {
        guard.generation = guard.generation.wrapping_add(1);
        guard.lookup = map;
        guard.actions = actions;
        LAST_GENERATION.store(guard.generation, Ordering::SeqCst);
    }
}

pub fn take_action_by_slot(slot: usize, generation: u64) -> Option<Action> {
    let guard = keymap_lock().read().ok()?;
    if guard.generation != generation {
        return None;
    }
    guard.actions.get(slot).cloned()
}

pub fn get_action_by_slot(slot: usize) -> Option<Action> {
    let guard = keymap_lock().read().ok()?;
    guard.actions.get(slot).cloned()
}

unsafe extern "system" fn keyboard_proc(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if code < 0 {
        return unsafe { CallNextHookEx(std::ptr::null_mut(), code, wparam, lparam) };
    }

    let kb = unsafe { &*(lparam as *const KBDLLHOOKSTRUCT) };
    if kb.dwExtraInfo == SYNTHETIC_EXTRA || (kb.flags & LLKHF_INJECTED) != 0 {
        return unsafe { CallNextHookEx(std::ptr::null_mut(), code, wparam, lparam) };
    }

    let msg = wparam as u32;
    let is_down = msg == WM_KEYDOWN || msg == WM_SYSKEYDOWN;
    let is_up = msg == WM_KEYUP || msg == WM_SYSKEYUP;
    let vk = kb.vkCode;

    if let Some(bit) = modifier_bit(vk) {
        if is_down {
            MOD_STATE.fetch_or(bit, Ordering::Relaxed);
        } else if is_up {
            MOD_STATE.fetch_and(!bit, Ordering::Relaxed);
        }
        return unsafe { CallNextHookEx(std::ptr::null_mut(), code, wparam, lparam) };
    }

    if is_up {
        if let Ok(mut set) = suppressed_set().try_lock() {
            if set.remove(&vk) {
                return 1;
            }
        }
        return unsafe { CallNextHookEx(std::ptr::null_mut(), code, wparam, lparam) };
    }

    if !is_down {
        return unsafe { CallNextHookEx(std::ptr::null_mut(), code, wparam, lparam) };
    }

    // Reconcile tracked modifier state with live GetAsyncKeyState so focus/UAC never leaves stuck bits.
    let mods = read_live_modifiers();
    MOD_STATE.store(mods, Ordering::Relaxed);

    if let Ok(guard) = keymap_lock().try_read() {
        let normalized_vk = normalize_numpad_or_oem_vk(vk);
        if let Some(&slot) = guard.lookup.get(&Chord {
            mods,
            vk: normalized_vk,
        }) {
            let action = &guard.actions[slot];
            let allowed_while_paused = matches!(action, Action::TogglePause | Action::Quit);
            if PAUSED.load(Ordering::Relaxed) && !allowed_while_paused {
                return unsafe { CallNextHookEx(std::ptr::null_mut(), code, wparam, lparam) };
            }

            // Never intercept reserved Win+L lock shortcut even if user somehow added it.
            if (mods & MOD_WIN) != 0 && vk == ('L' as u32) && (mods & !MOD_WIN) == 0 {
                return unsafe { CallNextHookEx(std::ptr::null_mut(), code, wparam, lparam) };
            }

            if (mods & MOD_WIN) != 0 {
                suppress_start_menu();
            }
            if let Ok(mut set) = suppressed_set().try_lock() {
                set.insert(vk);
            }
            let target = TARGET_HWND.load(Ordering::Relaxed) as HWND;
            if !target.is_null() {
                unsafe {
                    PostMessageW(target, WM_APP_ACTION, slot, guard.generation as isize);
                }
            }
            return 1;
        }
    }

    if let Ok(mut set) = suppressed_set().try_lock() {
        set.remove(&vk);
    }
    LAST_KEY_MS.store(now_ms(), Ordering::Relaxed);
    unsafe { CallNextHookEx(std::ptr::null_mut(), code, wparam, lparam) }
}

fn modifier_bit(vk: u32) -> Option<u8> {
    let v = vk as u16;
    if v == VK_LWIN || v == VK_RWIN {
        Some(MOD_WIN)
    } else if v == VK_LCONTROL || v == VK_RCONTROL {
        Some(MOD_CTRL)
    } else if v == VK_LSHIFT || v == VK_RSHIFT {
        Some(MOD_SHIFT)
    } else if v == VK_LMENU || v == VK_RMENU {
        Some(MOD_ALT)
    } else {
        None
    }
}

fn is_vk_down(vk: u16) -> bool {
    unsafe { (GetAsyncKeyState(vk as i32) as u16 & 0x8000) != 0 }
}

fn read_live_modifiers() -> u8 {
    let mut mods = 0u8;
    if is_vk_down(VK_LWIN) || is_vk_down(VK_RWIN) {
        mods |= MOD_WIN;
    }
    if is_vk_down(VK_LCONTROL) || is_vk_down(VK_RCONTROL) {
        mods |= MOD_CTRL;
    }
    if is_vk_down(VK_LSHIFT) || is_vk_down(VK_RSHIFT) {
        mods |= MOD_SHIFT;
    }
    if is_vk_down(VK_LMENU) || is_vk_down(VK_RMENU) {
        mods |= MOD_ALT;
    }
    mods
}

/// Injects an unassigned keystroke while `Win` is held so releasing `Win`
/// does not trigger the Windows Start Menu.
fn suppress_start_menu() {
    let inputs = [
        INPUT {
            r#type: INPUT_KEYBOARD,
            Anonymous: INPUT_0 {
                ki: KEYBDINPUT {
                    wVk: 0xFF,
                    wScan: 0,
                    dwFlags: 0,
                    time: 0,
                    dwExtraInfo: SYNTHETIC_EXTRA,
                },
            },
        },
        INPUT {
            r#type: INPUT_KEYBOARD,
            Anonymous: INPUT_0 {
                ki: KEYBDINPUT {
                    wVk: 0xFF,
                    wScan: 0,
                    dwFlags: KEYEVENTF_KEYUP,
                    time: 0,
                    dwExtraInfo: SYNTHETIC_EXTRA,
                },
            },
        },
    ];
    unsafe {
        SendInput(
            inputs.len() as u32,
            inputs.as_ptr(),
            size_of::<INPUT>() as i32,
        );
    }
}

fn normalize_numpad_or_oem_vk(vk: u32) -> u32 {
    if vk == VK_ADD as u32 {
        VK_OEM_PLUS as u32
    } else if vk == VK_SUBTRACT as u32 {
        VK_OEM_MINUS as u32
    } else {
        vk
    }
}

fn parse_chord(spec: &str) -> Option<Chord> {
    let mut mods = 0u8;
    let mut key_part: Option<&str> = None;

    for raw in spec.split('+') {
        let token = raw.trim();
        if token.is_empty() {
            continue;
        }
        match token.to_ascii_lowercase().as_str() {
            "win" | "super" | "meta" | "mod" => mods |= MOD_WIN,
            "ctrl" | "control" => mods |= MOD_CTRL,
            "shift" => mods |= MOD_SHIFT,
            "alt" => mods |= MOD_ALT,
            _ => {
                if key_part.is_some() {
                    return None;
                }
                key_part = Some(token);
            }
        }
    }

    let vk = parse_vk(key_part?)?;
    Some(Chord { mods, vk })
}

fn parse_vk(token: &str) -> Option<u32> {
    let lower = token.to_ascii_lowercase();
    if lower.len() == 1 {
        let ch = lower.bytes().next()?;
        if ch.is_ascii_alphabetic() {
            let upper = ch.to_ascii_uppercase();
            let code = upper as u32;
            if code >= VK_A as u32 && code <= VK_Z as u32 {
                return Some(code);
            }
        }
        if ch.is_ascii_digit() {
            let code = ch as u32;
            if code >= VK_0 as u32 && code <= VK_9 as u32 {
                return Some(code);
            }
        }
    }

    let vk = match lower.as_str() {
        "left" => VK_LEFT as u32,
        "right" => VK_RIGHT as u32,
        "up" => VK_UP as u32,
        "down" => VK_DOWN as u32,
        "space" => VK_SPACE as u32,
        "enter" | "return" => VK_RETURN as u32,
        "escape" | "esc" => VK_ESCAPE as u32,
        "tab" => VK_TAB as u32,
        "backspace" | "back" => VK_BACK as u32,
        "delete" | "del" => VK_DELETE as u32,
        "insert" | "ins" => VK_INSERT as u32,
        "home" => VK_HOME as u32,
        "end" => VK_END as u32,
        "pageup" | "pgup" | "prior" => VK_PRIOR as u32,
        "pagedown" | "pgdn" | "next" => VK_NEXT as u32,
        "equal" | "equals" | "plus" | "=" | "+" => VK_OEM_PLUS as u32,
        "minus" | "dash" | "-" => VK_OEM_MINUS as u32,
        "comma" | "," => VK_OEM_COMMA as u32,
        "period" | "dot" | "." => VK_OEM_PERIOD as u32,
        "bracketleft" | "lbracket" | "[" => VK_OEM_4 as u32,
        "backslash" | "\\" => VK_OEM_5 as u32,
        "bracketright" | "rbracket" | "]" => VK_OEM_6 as u32,
        _ => {
            if let Some(num) = lower.strip_prefix('f').and_then(|n| n.parse::<u32>().ok()) {
                if (1..=12).contains(&num) {
                    return Some((VK_F1 as u32) + num - 1);
                }
            }
            let _ = VK_F12;
            return None;
        }
    };
    Some(vk)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_default_bindings() {
        let cfg = winscroller2d_core::Config::default();
        for b in &cfg.bindings {
            assert!(
                parse_chord(&b.keys).is_some(),
                "failed to parse default binding {}",
                b.keys
            );
        }
    }
}
