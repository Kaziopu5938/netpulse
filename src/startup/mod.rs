use std::io;

use winreg::enums::{DELETE, HKEY_CURRENT_USER, KEY_READ, KEY_SET_VALUE};
use winreg::RegKey;

const RUN_KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";
const VALUE_NAME: &str = "NetPulse";

fn command() -> String {
    format!("\"{}\"", crate::platform::exe_path().display())
}

/// Reports enabled only when the Run entry actually points at this executable.
/// A stale or hijacked value must not be mistaken for our own autostart entry.
pub fn enabled() -> bool {
    registered_command()
        .map(|value| value.trim().eq_ignore_ascii_case(&command()))
        .unwrap_or(false)
}

fn registered_command() -> Option<String> {
    RegKey::predef(HKEY_CURRENT_USER)
        .open_subkey_with_flags(RUN_KEY, KEY_READ)
        .and_then(|key| key.get_value::<String, _>(VALUE_NAME))
        .ok()
}

/// Makes the registry match the stored preference, rewriting the entry when it
/// points at a different copy of the executable.
pub fn sync(desired: bool) -> io::Result<()> {
    let registered = registered_command();
    if desired {
        if registered.as_deref() == Some(command().as_str()) {
            Ok(())
        } else {
            set_enabled(true)
        }
    } else if registered.is_some() {
        set_enabled(false)
    } else {
        Ok(())
    }
}

pub fn set_enabled(on: bool) -> io::Result<()> {
    let current_user = RegKey::predef(HKEY_CURRENT_USER);
    if on {
        let key = current_user.open_subkey_with_flags(RUN_KEY, KEY_SET_VALUE)?;
        return key.set_value(VALUE_NAME, &command());
    }
    match current_user.open_subkey_with_flags(RUN_KEY, KEY_SET_VALUE | DELETE) {
        Ok(key) => match key.delete_value(VALUE_NAME) {
            Ok(()) => Ok(()),
            Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
            Err(error) => Err(error),
        },
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error),
    }
}
