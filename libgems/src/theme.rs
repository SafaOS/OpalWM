//! Provides some default theme keys
//!
//! ACCENT_COLOR_N starting from [`ACCENT_COLOR`]  to [`ACCENT_COLOR_4`] the bigger N is the darker from ACCENT_COLOR is the color.

use crate::{AppEnv, EnvKey};
pub use uopal_desktop::themes as utheme;
use uopal_desktop::themes::ThemesDatabase;

pub const DEFAULT_BUTTON_COLOR: EnvKey<super::Color> =
    EnvKey::new("built-in.theme.DEFAULT_BUTTON_COLOR");
pub const ACCENT_COLOR: EnvKey<super::Color> = EnvKey::new("built-in.theme.ACCENT_COLOR");
pub const ACCENT_COLOR_1: EnvKey<super::Color> = EnvKey::new("built-in.theme.ACCENT_COLOR_1");
pub const ACCENT_COLOR_2: EnvKey<super::Color> = EnvKey::new("built-in.theme.ACCENT_COLOR_2");
pub const ACCENT_COLOR_3: EnvKey<super::Color> = EnvKey::new("built-in.theme.ACCENT_COLOR_3");
pub const ACCENT_COLOR_4: EnvKey<super::Color> = EnvKey::new("built-in.theme.ACCENT_COLOR_4");

pub const BACKGROUND_COLOR: EnvKey<super::Color> = EnvKey::new("built-in.theme.BACKGROUND_COLOR");
pub const BACKGROUND_COLOR_1: EnvKey<super::Color> =
    EnvKey::new("built-in.theme.BACKGROUND_COLOR_1");

pub const DEFAULT_TEXT_COLOR: EnvKey<super::Color> =
    EnvKey::new("built-in.theme.DEFAULT_TEXT_COLOR");

fn apply_desktop_theme<'e>(theme: &utheme::Theme, app_env: &'e mut AppEnv) -> &'e mut AppEnv {
    app_env.set_key(DEFAULT_BUTTON_COLOR, theme.accent_color_2.into());

    app_env.set_key(ACCENT_COLOR, theme.accent_color.into());
    app_env.set_key(ACCENT_COLOR_1, theme.accent_color_1.into());
    app_env.set_key(ACCENT_COLOR_2, theme.accent_color_2.into());
    app_env.set_key(ACCENT_COLOR_3, theme.accent_color_3.into());
    app_env.set_key(ACCENT_COLOR_4, theme.accent_color_4.into());
    app_env.set_key(BACKGROUND_COLOR, theme.background_color.into());
    app_env.set_key(BACKGROUND_COLOR_1, theme.background_color_1.into());

    app_env.set_key(DEFAULT_TEXT_COLOR, theme.text_color.into());
    app_env
}

pub(crate) fn default_app_theme(app_env: &mut AppEnv) -> &mut AppEnv {
    apply_desktop_theme(
        &ThemesDatabase::try_load()
            .unwrap_or_else(|_| ThemesDatabase::placeholder())
            .apps_theme(),
        app_env,
    )
}

pub(crate) fn default_sys_theme(app_env: &mut AppEnv) -> &mut AppEnv {
    apply_desktop_theme(
        &ThemesDatabase::try_load()
            .unwrap_or_else(|_| ThemesDatabase::placeholder())
            .sys_theme(),
        app_env,
    )
}
