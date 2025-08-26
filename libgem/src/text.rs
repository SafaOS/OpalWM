use std::sync::{LazyLock, Mutex, MutexGuard};

use cosmic_text::{Attrs, Buffer, FontSystem, Metrics, SwashCache};
use libopal::window::Pixel;

const DEFAULT_FONT_DATA: &[u8] = include_bytes!("../../assets/DejaVuSansMono.ttf");
static SWASH_CACHE: LazyLock<Mutex<SwashCache>> = LazyLock::new(|| Mutex::new(SwashCache::new()));
static FONT_SYSTEM: LazyLock<Mutex<FontSystem>> = LazyLock::new(|| {
    Mutex::new(FontSystem::new_with_fonts([
        cosmic_text::fontdb::Source::Binary(std::sync::Arc::new(DEFAULT_FONT_DATA)),
    ]))
});

fn font_system() -> MutexGuard<'static, FontSystem> {
    FONT_SYSTEM
        .lock()
        .expect("Failed to get lock on font system")
}

pub use cosmic_text::Align;

pub struct Text {
    buffer: Buffer,
    color: Pixel,
    alignment: Option<Align>,
}

impl Text {
    pub fn width(&self) -> f32 {
        self.buffer
            .layout_runs()
            .map(|run| run.glyphs.iter().map(|glyph| glyph.w).sum::<f32>())
            .sum()
    }

    pub fn biggest_line_width(&self) -> f32 {
        let center = self.alignment == Some(Align::Center);

        let results = self
            .buffer
            .layout_runs()
            .map(|run| run.line_w)
            .max_by(|s, o| s.partial_cmp(o).unwrap_or(std::cmp::Ordering::Equal))
            .unwrap_or(0.0);
        let off = if center {
            if let (Some(width), _) = self.buffer.size() {
                (width - results) / 2.
            } else {
                0.
            }
        } else {
            0.
        };

        results + (off * 2.)
    }

    pub fn font_size(&self) -> f32 {
        self.buffer.metrics().font_size
    }

    pub fn height(&self) -> f32 {
        self.buffer.metrics().line_height * self.buffer.layout_runs().count() as f32
    }

    pub fn new(
        font_size: f32,
        line_height: f32,
        lines_max_height: Option<f32>,
        line_max_width: Option<f32>,
    ) -> Self {
        let mut font_system = font_system();

        let metrics = Metrics::new(font_size, line_height);
        let mut buffer = Buffer::new(&mut font_system, metrics);
        buffer.set_size(&mut font_system, line_max_width, lines_max_height);
        Self {
            buffer,
            alignment: None,
            color: Pixel::from_rgb(0xFF, 0xFF, 0xFF),
        }
    }

    pub fn set_size(&mut self, line_max_width: Option<f32>, lines_max_height: Option<f32>) {
        let font_system = &mut font_system();
        self.buffer
            .set_size(font_system, line_max_width, lines_max_height);
    }

    pub fn set_color(&mut self, color: Pixel) {
        self.color = color;
    }

    pub fn align(&mut self, alignment: Option<Align>) {
        self.alignment = alignment;
    }

    pub fn set_text(&mut self, text: &str) {
        let font_system = &mut font_system();
        self.buffer.set_text(
            font_system,
            text,
            &Attrs::new(),
            cosmic_text::Shaping::Basic,
        );

        let mut changed = false;
        for line in self.buffer.lines.iter_mut() {
            changed = changed | line.set_align(self.alignment);
        }

        if changed {
            self.buffer.shape_until_scroll(font_system, true);
        }
    }

    pub fn draw(&mut self, mut f: impl FnMut(i32, i32, u32, u32, Pixel)) {
        self.buffer.draw(
            &mut font_system(),
            &mut *SWASH_CACHE
                .lock()
                .expect("Failed to get lock on SWASH Cache"),
            cosmic_text::Color::rgba(
                self.color.red(),
                self.color.green(),
                self.color.blue(),
                self.color.alpha(),
            ),
            |x, y, width, height, color| {
                f(
                    x,
                    y,
                    width,
                    height,
                    Pixel::from_rgb(color.r(), color.g(), color.b()).with_alpha(color.a()),
                )
            },
        );
    }
}
