//! POSIX locale handling
use std::env;

/// Returns the default locale string.
pub fn get_locale() -> String {
    env::var_os("LC_ALL")
        .or_else(|| env::var_os("LANG"))
        .map_or(None, |os_string| os_string.into_string().ok())
        .unwrap_or_else(|| "C".into())
}

/// Returns the collate category locale string.
///
/// The collate category applies to sorting and regular expressions.
pub fn get_locale_collate() -> String {
    env::var_os("LC_ALL")
        .or_else(|| env::var_os("LC_COLLATE"))
        .or_else(|| env::var_os("LANG"))
        .map_or(None, |os_string| os_string.into_string().ok())
        .unwrap_or_else(|| "C".into())
}

/// Returns the ctype category locale string.
///
/// The ctype category applies to classification and conversion of characters
/// and to multibyte und wide charcters.
pub fn get_locale_ctype() -> String {
    env::var_os("LC_ALL")
        .or_else(|| env::var_os("LC_CTYPE"))
        .or_else(|| env::var_os("LANG"))
        .map_or(None, |os_string| os_string.into_string().ok())
        .unwrap_or_else(|| "C".into())
}

/// Returns the messages category locale string.
///
/// The message category applies to selecting the language used in the user
/// interface for message translation and contains regular expressions for
/// affirmative and negative responses.
pub fn get_locale_messages() -> String {
    env::var_os("LC_ALL")
        .or_else(|| env::var_os("LC_MESSAGES"))
        .or_else(|| env::var_os("LANG"))
        .map_or(None, |os_string| os_string.into_string().ok())
        .unwrap_or_else(|| "C".into())
}

/// Returns the monetary category locale string.
///
/// The monetary category applies to formatting monetary values.
pub fn get_locale_monetary() -> String {
    env::var_os("LC_ALL")
        .or_else(|| env::var_os("LC_MONETARY"))
        .or_else(|| env::var_os("LANG"))
        .map_or(None, |os_string| os_string.into_string().ok())
        .unwrap_or_else(|| "C".into())
}

/// Returns the numeric category locale string.
///
/// The numeric category applies to formatting numeric values that are
/// not monetary.
pub fn get_locale_numeric() -> String {
    env::var_os("LC_ALL")
        .or_else(|| env::var_os("LC_NUMERIC"))
        .or_else(|| env::var_os("LANG"))
        .map_or(None, |os_string| os_string.into_string().ok())
        .unwrap_or_else(|| "C".into())
}

/// Returns the time category locale string.
///
/// The time category applies to formatting date and time values.
pub fn get_locale_time() -> String {
    env::var_os("LC_ALL")
        .or_else(|| env::var_os("LC_TIME"))
        .or_else(|| env::var_os("LANG"))
        .map_or(None, |os_string| os_string.into_string().ok())
        .unwrap_or_else(|| "C".into())
}
