use super::Color;
use crate::{AppEnv, EnvKey};

pub const DEFAULT_BUTTON_COLOR: EnvKey<super::Color> =
    EnvKey::new("built-in.theme.DEFAULT_BUTTON_COLOR");
pub const DEFAULT_TEXT_COLOR: EnvKey<super::Color> =
    EnvKey::new("built-in.theme.DEFAULT_TEXT_COLOR");

pub(crate) fn default_theme(app_env: &mut AppEnv) -> &mut AppEnv {
    app_env
        .set_key(DEFAULT_BUTTON_COLOR, Color::rgb(0xFD, 0xB0, 0xC0))
        .set_key(DEFAULT_TEXT_COLOR, Color::BLACK)
}
