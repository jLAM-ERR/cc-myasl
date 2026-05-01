//! State classification + colour/icon picking driven by env vars.
//!
//! `STATUSLINE_RED` (default 20) and `STATUSLINE_YELLOW` (default 50)
//! set the thresholds used by both ANSI colour and emoji icon rendering.

/// State of a quota value, derived from the remaining percentage.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum State {
    Green,
    Yellow,
    Red,
    Unknown,
}

/// Classify a remaining-percentage value into a [`State`].
///
/// Reads `STATUSLINE_RED` (default `20`) and `STATUSLINE_YELLOW`
/// (default `50`) from the environment on every call — no caching.
///
/// - `None` → [`State::Unknown`]
/// - `left < red` → [`State::Red`]
/// - `left < yellow` → [`State::Yellow`]
/// - otherwise → [`State::Green`]
pub fn classify(left: Option<f64>) -> State {
    let Some(left) = left else {
        return State::Unknown;
    };

    let red = std::env::var("STATUSLINE_RED")
        .ok()
        .and_then(|s| s.parse::<f64>().ok())
        .unwrap_or(20.0);

    let yellow = std::env::var("STATUSLINE_YELLOW")
        .ok()
        .and_then(|s| s.parse::<f64>().ok())
        .unwrap_or(50.0);

    if left < red {
        State::Red
    } else if left < yellow {
        State::Yellow
    } else {
        State::Green
    }
}

/// Return the ANSI colour escape for the given [`State`].
///
/// - Green  → `"\x1b[32m"`
/// - Yellow → `"\x1b[33m"`
/// - Red    → `"\x1b[31m"`
/// - Unknown → `""` (no colour)
pub fn pick_color(s: State) -> &'static str {
    match s {
        State::Green => "\x1b[32m",
        State::Yellow => "\x1b[33m",
        State::Red => "\x1b[31m",
        State::Unknown => "",
    }
}

/// Return an emoji indicator for the given [`State`].
///
/// - Green  → `"🟢"`
/// - Yellow → `"🟡"`
/// - Red    → `"🔴"`
/// - Unknown → `"⚪"`
pub fn pick_icon(s: State) -> &'static str {
    match s {
        State::Green => "🟢",
        State::Yellow => "🟡",
        State::Red => "🔴",
        State::Unknown => "⚪",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::format::ENV_MUTEX as ENV_LOCK;

    #[test]
    fn classify_none_is_unknown() {
        assert_eq!(classify(None), State::Unknown);
    }

    #[test]
    fn classify_default_thresholds() {
        // default red=20, yellow=50
        let _guard = ENV_LOCK.lock().unwrap();
        unsafe { std::env::remove_var("STATUSLINE_RED") };
        unsafe { std::env::remove_var("STATUSLINE_YELLOW") };

        assert_eq!(classify(Some(10.0)), State::Red);
        assert_eq!(classify(Some(30.0)), State::Yellow);
        assert_eq!(classify(Some(70.0)), State::Green);
    }

    #[test]
    fn classify_boundary_at_default_red() {
        let _guard = ENV_LOCK.lock().unwrap();
        unsafe { std::env::remove_var("STATUSLINE_RED") };
        unsafe { std::env::remove_var("STATUSLINE_YELLOW") };

        // exactly at red threshold → still Yellow (not < red)
        assert_eq!(classify(Some(20.0)), State::Yellow);
        // just below red → Red
        assert_eq!(classify(Some(19.9)), State::Red);
    }

    #[test]
    fn classify_env_override() {
        let _guard = ENV_LOCK.lock().unwrap();
        // Override: red=30, yellow=60
        unsafe { std::env::set_var("STATUSLINE_RED", "30") };
        unsafe { std::env::set_var("STATUSLINE_YELLOW", "60") };

        assert_eq!(classify(Some(25.0)), State::Red); // 25 < 30
        assert_eq!(classify(Some(45.0)), State::Yellow); // 30 <= 45 < 60
        assert_eq!(classify(Some(75.0)), State::Green); // >= 60

        unsafe { std::env::remove_var("STATUSLINE_RED") };
        unsafe { std::env::remove_var("STATUSLINE_YELLOW") };
    }

    #[test]
    fn classify_invalid_env_falls_back_to_default() {
        let _guard = ENV_LOCK.lock().unwrap();
        unsafe { std::env::set_var("STATUSLINE_RED", "not_a_number") };
        unsafe { std::env::set_var("STATUSLINE_YELLOW", "also_bad") };

        // Should fall back to default red=20, yellow=50
        assert_eq!(classify(Some(10.0)), State::Red);
        assert_eq!(classify(Some(30.0)), State::Yellow);
        assert_eq!(classify(Some(70.0)), State::Green);

        unsafe { std::env::remove_var("STATUSLINE_RED") };
        unsafe { std::env::remove_var("STATUSLINE_YELLOW") };
    }

    #[test]
    fn pick_color_returns_correct_ansi() {
        assert_eq!(pick_color(State::Green), "\x1b[32m");
        assert_eq!(pick_color(State::Yellow), "\x1b[33m");
        assert_eq!(pick_color(State::Red), "\x1b[31m");
        assert_eq!(pick_color(State::Unknown), "");
    }

    #[test]
    fn pick_icon_returns_correct_emoji() {
        assert_eq!(pick_icon(State::Green), "🟢");
        assert_eq!(pick_icon(State::Yellow), "🟡");
        assert_eq!(pick_icon(State::Red), "🔴");
        assert_eq!(pick_icon(State::Unknown), "⚪");
    }
}
