use std::io;

use winreg::enums::{HKEY_CURRENT_USER, KEY_READ, KEY_SET_VALUE, DELETE};
use winreg::RegKey;

const RUN_KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";
const VALUE_NAME: &str = "NetPulse";

fn command() -> String {
    format!("\"{}\"", crate::platform::exe_path().display())
}

pub fn enabled() -> bool {
    RegKey::predef(HKEY_CURRENT_USER)
        .open_subkey_with_flags(RUN_KEY, KEY_READ)
        .and_then(|key| key.get_value::<String, _>(VALUE_NAME))
        .map(|value| !value.trim().is_empty())
        .unwrap_or(false)
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
