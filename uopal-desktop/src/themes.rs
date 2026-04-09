use std::{
    collections::HashMap,
    fmt,
    fs::File,
    io,
    path::{Path, PathBuf},
    sync::Arc,
};

use serde::{Deserialize, Serialize, de, ser};

/// Path of the color themes directory, relative to the user configuration directory.
pub static THEMES_SUBPATH: &str = "themes";
pub static THEME_JSON_PATH: &str = "theme.json";
/// Path of the wallpaper directory. relative to the user configuration directory.
pub static WALLPAPERS_SUBPATH: &str = "pictures/wallpapers";

/// Given a relative path to an unkown configuration directory,
///
/// returns the full path in all configuration that affects the current user in prioritized order.
pub fn lookup_configuration_path(path: impl AsRef<Path>) -> Vec<PathBuf> {
    const SYS_USR: &str = "sys:/usr";
    let sys_usr: &Path = SYS_USR.as_ref();
    let path = sys_usr.join(path.as_ref());
    if path.exists() {
        Vec::from([path])
    } else {
        Vec::new()
    }
}

struct ColorVisitor;

impl<'de> de::Visitor<'de> for ColorVisitor {
    type Value = Color;

    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "a hex color string like \"0xff00ff\"")
    }

    fn visit_str<E: de::Error>(self, v: &str) -> Result<Color, E> {
        let hex = v
            .strip_prefix("0x")
            .ok_or_else(|| E::invalid_value(de::Unexpected::Str(v), &self))?;

        let has_alpha = hex.len() > (2 * 3);
        let num = u32::from_str_radix(hex, 16)
            .map_err(|_| E::invalid_value(de::Unexpected::Str(v), &self))?;

        Ok(if has_alpha {
            Color::hex_rgba(num)
        } else {
            Color::hex_rgb(num)
        })
    }
}

/// Describes an RGB Color.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl<'de> de::Deserialize<'de> for Color {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_str(ColorVisitor)
    }
}

impl<'se> ser::Serialize for Color {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: ser::Serializer,
    {
        use std::io::Write;
        let mut buf = [0u8; 16];
        let has_alpha = self.a != 0xFF;
        let wrote = if has_alpha {
            write!(&mut buf[..], "{:#010x}", self.to_rgba()).map(|()| 2 + (2 * 4))
        } else {
            write!(&mut buf[..], "{:#08x}", self.to_rgb()).map(|()| 2 + (2 * 3))
        }
        .expect("Failed to format 32bit hex in a 16byte buffer");
        let s = str::from_utf8(&buf[..wrote]).expect("Expected utf8 formatted string");
        serializer.serialize_str(s)
    }
}

impl Color {
    pub const BLACK: Self = Self::rgb(0, 0, 0);
    pub const WHITE: Self = Self::rgb(0xFF, 0xFF, 0xFF);

    #[inline]
    /// Constructs a new RGBA Color.
    pub const fn rgba(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self { r, g, b, a }
    }

    #[inline]
    /// Constructs a new RGB Color.
    pub const fn rgb(r: u8, g: u8, b: u8) -> Self {
        Self::rgba(r, g, b, 0xFF)
    }

    #[inline]
    /// Constructs a new RGB Color from a hex value.
    pub const fn hex_rgb(hex: u32) -> Self {
        let r = (hex >> 16) as u8;
        let g = (hex >> 8) as u8;
        let b = hex as u8;
        Self::rgb(r, g, b)
    }

    #[inline]
    /// Constructs a new RGB Color from a hex value.
    pub const fn hex_rgba(hex: u32) -> Self {
        let r = (hex >> 24) as u8;
        let g = (hex >> 16) as u8;
        let b = (hex >> 8) as u8;
        let a = hex as u8;
        Self::rgba(r, g, b, a)
    }

    pub const fn to_rgba(self) -> u32 {
        ((self.r as u32) << 24) | ((self.g as u32) << 16) | ((self.b as u32) << 8) | self.a as u32
    }

    pub const fn to_rgb(self) -> u32 {
        ((self.r as u32) << 16) | ((self.g as u32) << 8) | (self.b as u32)
    }
}

/// Raw unprocessed theme json.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RawTheme {
    background_path: Option<PathBuf>,
    background_color: Option<Color>,
    #[serde(rename = "backgroundColor_1")]
    background_color_1: Option<Color>,
    accent_color: Option<Color>,
    #[serde(rename = "accentColor_1")]
    accent_color_1: Option<Color>,
    #[serde(rename = "accentColor_2")]
    accent_color_2: Option<Color>,
    #[serde(rename = "accentColor_3")]
    accent_color_3: Option<Color>,
    #[serde(rename = "accentColor_4")]
    accent_color_4: Option<Color>,
    text_color: Option<Color>,
}

impl RawTheme {
    #[inline]
    /// Processes that theme replacing all uninitialized fields with some defaults.
    pub fn unwrap_or_default(&self) -> Theme {
        self.merge_or_default(&RawTheme::default())
    }

    /// Merge this theme with other theme, by replacing non-existing fields with it or if none has the field, a default value is provided.
    pub fn merge_or_default(&self, other: &RawTheme) -> Theme {
        Theme {
            background_path: self
                .background_path
                .clone()
                .or_else(|| other.background_path.clone()),
            background_color: self
                .background_color
                .or(other.background_color)
                .unwrap_or(Color::hex_rgb(0xebdbb2)),
            background_color_1: self
                .background_color_1
                .or(other.background_color_1)
                .unwrap_or(Color::hex_rgb(0xf0bbb3)),
            accent_color: self
                .accent_color
                .or(other.accent_color)
                .unwrap_or(Color::hex_rgb(0xfdb0c0)),
            accent_color_1: self
                .accent_color_1
                .or(other.accent_color_1)
                .unwrap_or(Color::hex_rgb(0xfb7cb7)),
            accent_color_2: self
                .accent_color_2
                .or(other.accent_color_2)
                .unwrap_or(Color::hex_rgb(0xfb7cb7)),
            accent_color_3: self
                .accent_color_3
                .or(other.accent_color_3)
                .unwrap_or(Color::hex_rgb(0xfb7cb7)),
            accent_color_4: self
                .accent_color_4
                .or(other.accent_color_4)
                .unwrap_or(Color::hex_rgb(0xfb7cb7)),
            text_color: self.text_color.or(other.text_color).unwrap_or(Color::BLACK),
        }
    }
}

/// Describes a Theme.
#[derive(Debug)]
pub struct Theme {
    pub background_path: Option<PathBuf>,
    pub background_color: Color,
    pub background_color_1: Color,
    pub accent_color: Color,
    pub accent_color_1: Color,
    pub accent_color_2: Color,
    pub accent_color_3: Color,
    pub accent_color_4: Color,
    pub text_color: Color,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct RawThemesDatabase {
    current_apps: Option<String>,
    current_system: Option<String>,
    #[serde(default = "Default::default")]
    overrides: RawTheme,
}

/// Describes the currently loaded themes.
#[derive(Debug)]
pub struct ThemesDatabase {
    loaded_themes: HashMap<Arc<str>, RawTheme>,
    current_apps: Arc<str>,
    current_system: Arc<str>,
    overrides: RawTheme,
}

impl ThemesDatabase {
    /// Returns a placeholder theme database.
    pub fn placeholder() -> Self {
        let palceholder: Arc<str> = Arc::from("placeholder-theme");
        Self {
            loaded_themes: HashMap::from([(palceholder.clone(), RawTheme::default())]),
            current_apps: palceholder.clone(),
            current_system: palceholder,
            overrides: RawTheme::default(),
        }
    }

    /// Attempts to load the Themes database from the system.
    pub fn try_load() -> io::Result<Self> {
        let themes_dirs = lookup_configuration_path(THEMES_SUBPATH);
        let themes_dir = themes_dirs.get(0).ok_or(io::Error::new(
            io::ErrorKind::NotFound,
            "No themes directory found",
        ))?;

        let themes_dir_read = std::fs::read_dir(themes_dir)?;
        let themes_paths = themes_dir_read.filter_map(|e| e.ok()).map(|e| e.path());

        let database_file = File::open(themes_dir.join("theme.json"))?;
        let raw_database: RawThemesDatabase = serde_json::from_reader(database_file)?;

        let mut arc_apps_name: Option<Arc<str>> = None;
        let mut arc_sys_name: Option<Arc<str>> = None;
        let mut loaded_themes: HashMap<Arc<str>, RawTheme> = HashMap::new();

        for path in themes_paths {
            let Some(name) = path.file_name().and_then(|f| f.to_str()) else {
                continue;
            };

            if !name.ends_with(".json") || name == "theme.json" {
                continue;
            }

            let file = File::open(&path)?;
            let raw_theme: RawTheme = serde_json::from_reader(file)?;
            let arc_name: Arc<str> = Arc::from(name);

            if raw_database
                .current_apps
                .as_ref()
                .is_some_and(|n| n == name)
            {
                arc_apps_name = Some(arc_name.clone());
            }

            if raw_database
                .current_system
                .as_ref()
                .is_some_and(|n| n == name)
            {
                arc_sys_name = Some(arc_name.clone());
            }
            loaded_themes.insert(arc_name, raw_theme);
        }

        let first = loaded_themes
            .keys()
            .next()
            .ok_or(io::Error::new(io::ErrorKind::Other, "No themes loaded"))?
            .clone();

        let arc_apps_name = arc_apps_name.unwrap_or_else(|| first.clone());
        let arc_sys_name = arc_sys_name.unwrap_or(first);
        Ok(Self {
            loaded_themes,
            current_apps: arc_apps_name,
            current_system: arc_sys_name,
            overrides: raw_database.overrides,
        })
    }

    /// Returns a list of raw themes and their names.
    #[inline]
    pub fn themes(&self) -> impl Iterator<Item = (&str, &RawTheme)> {
        self.loaded_themes.iter().map(|(s, r)| (&**s, r))
    }

    /// Lookup a theme by name.
    #[inline]
    pub fn lookup_theme(&self, by_name: &str) -> Option<&RawTheme> {
        self.loaded_themes.get(by_name)
    }

    /// Returns the raw overrides applied to the current Theme.
    #[inline]
    pub const fn theme_overrides(&self) -> &RawTheme {
        &self.overrides
    }

    /// Returns the name of the Apps theme.
    #[inline]
    pub fn apps_theme_name(&self) -> &str {
        &self.current_apps
    }

    /// Returns the name of the System theme.
    #[inline]
    pub fn sys_theme_name(&self) -> &str {
        &self.current_system
    }

    /// Return the apps Theme.
    pub fn apps_theme(&self) -> Theme {
        self.overrides.merge_or_default(
            self.loaded_themes
                .get(&self.current_apps)
                .expect("Current App Theme is non-existent"),
        )
    }

    /// Returns the system Theme.
    pub fn sys_theme(&self) -> Theme {
        self.overrides.merge_or_default(
            self.loaded_themes
                .get(&self.current_system)
                .expect("Current System Theme is non-existent"),
        )
    }
}
