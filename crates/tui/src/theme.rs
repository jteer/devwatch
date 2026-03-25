//! Built-in colour themes for the TUI.
//!
//! Add a `theme = "dark"` (or `"light"` / `"high-contrast"`) line to
//! `config.toml` to switch themes.  The config editor also lets you cycle
//! through them interactively.

use ratatui::style::Color;

pub struct Theme {
    // Structure / chrome
    pub header:        Color, // column headers
    pub selected:      Color, // highlighted row
    pub dim:           Color, // de-emphasised text (age, timestamps, …)

    // PR table
    pub draft:         Color, // [draft] badge
    pub state_open:    Color, // "open" state
    pub state_merged:  Color, // "merged" state
    pub state_closed:  Color, // "closed" state

    // Event log
    pub event_new:     Color, // ● new PR
    pub event_upd:     Color, // ◆ updated PR
    pub event_clo:     Color, // ○ closed PR

    // Status bar
    pub status_ok:     Color, // Connected / polling done
    pub status_warn:   Color, // Connecting / Polling…
    pub status_err:    Color, // Disconnected
    pub status_demo:   Color, // Demo mode indicator
}

impl Theme {
    /// Dark terminal background (default).
    /// Uses `Gray` (not `DarkGray`) for dim text so it stays readable.
    pub fn dark() -> Self {
        Self {
            header:       Color::Cyan,
            selected:     Color::Yellow,
            dim:          Color::Gray,        // lighter than DarkGray

            draft:        Color::Gray,
            state_open:   Color::Green,
            state_merged: Color::Magenta,
            state_closed: Color::Red,

            event_new:    Color::Green,
            event_upd:    Color::Yellow,
            event_clo:    Color::Red,

            status_ok:    Color::Green,
            status_warn:  Color::Yellow,
            status_err:   Color::Red,
            status_demo:  Color::Magenta,
        }
    }

    /// Light terminal background.
    /// Swaps bright colours for darker equivalents that show on white/cream.
    pub fn light() -> Self {
        Self {
            header:       Color::Blue,
            selected:     Color::Magenta,
            dim:          Color::DarkGray,

            draft:        Color::DarkGray,
            state_open:   Color::Green,
            state_merged: Color::Magenta,
            state_closed: Color::Red,

            event_new:    Color::Green,
            event_upd:    Color::Yellow,
            event_clo:    Color::Red,

            status_ok:    Color::Green,
            status_warn:  Color::Yellow,
            status_err:   Color::Red,
            status_demo:  Color::Magenta,
        }
    }

    /// High-contrast: no dimming, everything at full brightness.
    /// Useful for accessibility or very low-contrast terminal emulators.
    pub fn high_contrast() -> Self {
        Self {
            header:       Color::Cyan,
            selected:     Color::Yellow,
            dim:          Color::White,       // no dimming at all

            draft:        Color::White,
            state_open:   Color::Green,
            state_merged: Color::Magenta,
            state_closed: Color::Red,

            event_new:    Color::Green,
            event_upd:    Color::Yellow,
            event_clo:    Color::Red,

            status_ok:    Color::Green,
            status_warn:  Color::Yellow,
            status_err:   Color::Red,
            status_demo:  Color::Magenta,
        }
    }

    pub fn from_name(name: &str) -> Self {
        match name.to_lowercase().as_str() {
            "light"                      => Self::light(),
            "high-contrast" | "high_contrast" => Self::high_contrast(),
            _                            => Self::dark(),
        }
    }

    /// Cycle to the next theme name (for the config editor picker).
    pub fn next_name(current: &str) -> &'static str {
        match current.to_lowercase().as_str() {
            "dark"          => "light",
            "light"         => "high-contrast",
            _               => "dark",
        }
    }
}
