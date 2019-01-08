//! Uses `xkbcommon` to keep track of keyboard state
use crate::locale::get_locale_ctype;
use std::os::unix::io::RawFd;
use xkbcommon::xkb::compose::{FeedResult, Status as ComposeStatus};
use xkbcommon::xkb::compose::{State as ComposeState, Table as ComposeTable};
use xkbcommon::xkb::compose::{COMPILE_NO_FLAGS, STATE_NO_FLAGS};
pub use xkbcommon::xkb::{keysyms, Keycode, Keysym};
use xkbcommon::xkb::{Context, Keymap, State};
use xkbcommon::xkb::{CONTEXT_NO_FLAGS, KEYMAP_COMPILE_NO_FLAGS};
use xkbcommon::xkb::{KEYMAP_FORMAT_TEXT_V1, STATE_MODS_EFFECTIVE};
use xkbcommon::xkb::{
    MOD_NAME_ALT, MOD_NAME_CAPS, MOD_NAME_CTRL, MOD_NAME_LOGO, MOD_NAME_NUM,
    MOD_NAME_SHIFT,
};

/// The state of a keyboard
pub struct KeyboardState {
    context: Context,
    keymap: Option<Keymap>,
    state: Option<State>,
    _compose_table: ComposeTable,
    compose_state: ComposeState,
}

impl KeyboardState {
    /// Creates a new `KeyboardState`
    pub fn new() -> Self {
        let locale = get_locale_ctype();
        let context = Context::new(CONTEXT_NO_FLAGS);
        let compose_table = ComposeTable::new_from_locale(
            &context,
            locale.as_str(),
            COMPILE_NO_FLAGS,
        )
        .unwrap();
        let compose_state = ComposeState::new(&compose_table, STATE_NO_FLAGS);
        KeyboardState {
            context,
            keymap: None,
            state: None,
            _compose_table: compose_table,
            compose_state,
        }
    }

    /// Loads a keymap from a file descriptor
    pub fn load_keymap_from_fd(&mut self, fd: RawFd, size: usize) {
        let keymap = Keymap::new_from_fd(
            &self.context,
            fd,
            size,
            KEYMAP_FORMAT_TEXT_V1,
            KEYMAP_COMPILE_NO_FLAGS,
        )
        .unwrap();
        let state = State::new(&keymap);
        self.keymap = Some(keymap);
        self.state = Some(state);
    }

    /// Updates the keyboard modifiers
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
        ModifiersState::from_xkb_state(&self.state.as_ref().unwrap())
    }

    /// Gets the keysym of a keycode
    pub fn get_sym(&mut self, rawkey: Keycode) -> Keysym {
        self.state.as_mut().unwrap().key_get_one_sym(rawkey + 8)
    }

    /// Gets the utf8 representation of a keycode if one exists
    pub fn get_utf8(&mut self, rawkey: Keycode) -> Option<String> {
        let utf8 = self.state.as_mut().unwrap().key_get_utf8(rawkey + 8);
        if utf8.is_empty() {
            None
        } else {
            Some(utf8)
        }
    }

    /// Determine whether a key should repeat or not
    pub fn key_repeats(&self, rawkey: Keycode) -> bool {
        self.keymap.as_ref().unwrap().key_repeats(rawkey + 8)
    }

    /// Feeds the compose state machine
    pub fn compose(&mut self, keysym: Keysym) -> Result<Option<String>, ()> {
        match self.compose_state.feed(keysym) {
            FeedResult::Accepted => match self.compose_state.status() {
                ComposeStatus::Nothing => Err(()),
                ComposeStatus::Composed => Ok(self.compose_state.utf8()),
                _ => Ok(None),
            },
            FeedResult::Ignored => Err(()),
        }
    }
}

unsafe impl Send for KeyboardState {}

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
    fn from_xkb_state(state: &State) -> ModifiersState {
        ModifiersState {
            ctrl: state
                .mod_name_is_active(&MOD_NAME_CTRL, STATE_MODS_EFFECTIVE),
            alt: state.mod_name_is_active(&MOD_NAME_ALT, STATE_MODS_EFFECTIVE),
            shift: state
                .mod_name_is_active(&MOD_NAME_SHIFT, STATE_MODS_EFFECTIVE),
            caps_lock: state
                .mod_name_is_active(&MOD_NAME_CAPS, STATE_MODS_EFFECTIVE),
            logo: state
                .mod_name_is_active(&MOD_NAME_LOGO, STATE_MODS_EFFECTIVE),
            num_lock: state
                .mod_name_is_active(&MOD_NAME_NUM, STATE_MODS_EFFECTIVE),
        }
    }
}
