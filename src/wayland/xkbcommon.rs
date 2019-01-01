use std::fs::File;
use xkbcommon::xkb::{Context, Keymap, KeymapFormat, State};
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

    pub fn load_keymap_from_file(
        &mut self,
        format: KeymapFormat,
        mut file: File,
    ) {
        let keymap = Keymap::new_from_file(
            &self.context,
            &mut file,
            format,
            KEYMAP_COMPILE_NO_FLAGS,
        ).unwrap();
        let state = State::new(&keymap);
        self.keymap = Some(keymap);
        self.state = Some(state);
    }

    pub fn set_repeat_info(&mut self, rate: u32, delay: u32) {
        self.repeat_info = Some(RepeatInfo::new(rate, delay));
    }

    pub fn set_modifiers(
        &mut self,
        mods_depressed: u32,
        mods_latched: u32,
        mods_locked: u32,
        group: u32,
    ) {

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
