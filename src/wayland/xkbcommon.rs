use std::os::unix::io::RawFd;
use xkbcommon::xkb::{Context, Keymap, State};
pub use xkbcommon::xkb::{Keycode, Keysym};
use xkbcommon::xkb::KEYMAP_FORMAT_TEXT_V1;
use xkbcommon::xkb::{CONTEXT_NO_FLAGS, KEYMAP_COMPILE_NO_FLAGS};
use xkbcommon::xkb::compose::{Table as ComposeTable, State as ComposeState};
use xkbcommon::xkb::compose::{COMPILE_NO_FLAGS, STATE_NO_FLAGS};
use crate::locale::get_locale_ctype;

pub struct KbState {
    context: Context,
    keymap: Option<Keymap>,
    state: Option<State>,
    compose_table: ComposeTable,
    compose_state: ComposeState,
    modifiers_state: ModifiersState,
    repeat_info: Option<RepeatInfo>,
}

impl KbState {
    pub fn new() -> Self {
        let locale = get_locale_ctype();
        let context = Context::new(CONTEXT_NO_FLAGS);
        let compose_table = ComposeTable::new_from_locale(
            &context,
            locale.as_str(),
            COMPILE_NO_FLAGS,
        ).unwrap();
        let compose_state = ComposeState::new(&compose_table, STATE_NO_FLAGS);
        let modifiers_state = ModifiersState::new();
        KbState {
            context,
            keymap: None,
            state: None,
            compose_table,
            compose_state,
            modifiers_state,
            repeat_info: None,
        }
    }

    pub fn load_keymap_from_fd(
        &mut self,
        fd: RawFd,
        size: usize,
    ) {
        let keymap = Keymap::new_from_fd(
            &self.context,
            fd,
            size,
            KEYMAP_FORMAT_TEXT_V1,
            KEYMAP_COMPILE_NO_FLAGS,
        ).unwrap();
        let state = State::new(&keymap);
        self.keymap = Some(keymap);
        self.state = Some(state);
    }

    pub fn set_repeat_info(&mut self, rate: u32, delay: u32) {
        self.repeat_info = Some(RepeatInfo::new(rate, delay));
    }

    pub fn update_modifiers(
        &mut self,
        mods_depressed: u32,
        mods_latched: u32,
        mods_locked: u32,
        group: u32,
    ) -> ModifiersState {
        self.state.as_mut().unwrap().update_mask(
            mods_depressed,
            mods_latched,
            mods_locked,
            0,
            0,
            group,
        );
        // TODO update modifiers_state
        self.modifiers_state.clone()
    }

    pub fn get_sym(&mut self, rawkey: Keycode) -> Keysym {
        self.state.as_mut().unwrap().key_get_one_sym(rawkey + 8)
    }

    pub fn get_utf8(&mut self, rawkey: Keycode) -> String {
        self.state.as_mut().unwrap().key_get_utf8(rawkey + 8)
    }
}

unsafe impl Send for KbState {}

/// Represents the current state of the keyboard modifiers
///
/// Each field of this struct represents a modifier and is `true` if this modifier is active.
///
/// For some modifiers, this means that the key is currently pressed, others are toggled
/// (like caps lock).
#[derive(Copy, Clone, Debug, Default)]
pub struct ModifiersState {
    /// The "control" key
    pub ctrl: bool,
    /// The "alt" key
    pub alt: bool,
    /// The "shift" key
    pub shift: bool,
    /// The "Caps lock" key
    pub caps_lock: bool,
    /// The "logo" key
    ///
    /// Also known as the "windows" key on most keyboards
    pub logo: bool,
    /// The "Num lock" key
    pub num_lock: bool,
}

impl ModifiersState {
    pub fn new() -> Self {
        ModifiersState::default()
    }
}

pub struct RepeatInfo {
    rate: u32,
    delay: u32,
}

impl RepeatInfo {
    pub fn new(rate: u32, delay: u32) -> Self {
        RepeatInfo {
            rate,
            delay,
        }
    }
}
