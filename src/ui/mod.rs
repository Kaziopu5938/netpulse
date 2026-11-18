pub mod window;

pub fn run() {
    if let Err(error) = window::run() {
        crate::platform::error(
            "NetPulse Settings",
            &format!("Could not open the settings window: {error}"),
        );
    }
}
