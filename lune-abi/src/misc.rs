/// Describes the allowed Audio channel count i.e mono or stero or other.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum ChannelCount {
    Single = 1,
    Dual = 2,
}

impl ChannelCount {
    pub const fn from_raw(raw: u8) -> Option<Self> {
        match raw {
            1 => Some(Self::Single),
            2 => Some(Self::Dual),
            _ => None,
        }
    }
}

/// Describes the allowed bits per sample. i.e audio bit depth.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum BitDepth {
    D16 = 16,
    D24 = 24,
    D32 = 32,
}

impl BitDepth {
    pub const fn from_raw(raw: u8) -> Option<Self> {
        match raw {
            16 => Some(Self::D16),
            24 => Some(Self::D24),
            32 => Some(Self::D32),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum SampleFormat {
    Singed = 0,
    Floating = 1,
}

impl SampleFormat {
    pub const fn from_raw(raw: u8) -> Option<Self> {
        match raw {
            0 => Some(Self::Singed),
            1 => Some(Self::Floating),
            _ => None,
        }
    }
}

/// Attempt to not avoid invalid states.
///
/// Represents the type of each sample.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i8)]
pub enum SampleType {
    I16 = 16,
    I24 = 24,
    I32 = 32,
    F32 = 33,
}

impl SampleType {
    pub const fn bit_depth(&self) -> BitDepth {
        match self {
            Self::I16 => BitDepth::D16,
            Self::I24 => BitDepth::D24,
            Self::I32 | Self::F32 => BitDepth::D32,
        }
    }

    pub const fn sample_format(&self) -> SampleFormat {
        match self {
            Self::I16 | Self::I24 | Self::I32 => SampleFormat::Singed,
            Self::F32 => SampleFormat::Floating,
        }
    }
}

/// Describes an Audio Stream.
#[derive(Debug, Clone, Copy)]
pub struct AudioFormat {
    channels: ChannelCount,
    freq_hz: u32,
    sample_type: SampleType,
}

impl AudioFormat {
    pub const fn from_raw(
        channels: u8,
        freq_hz: u32,
        bit_depth: u8,
        sample_kind: SampleFormat,
    ) -> Option<Self> {
        let Some(channels) = ChannelCount::from_raw(channels) else {
            return None;
        };
        let Some(bit_depth) = BitDepth::from_raw(bit_depth) else {
            return None;
        };
        Self::new(channels, freq_hz, bit_depth, sample_kind)
    }

    /// Attempts to construct a valid audio format.
    pub const fn new(
        channels: ChannelCount,
        freq_hz: u32,
        bit_depth: BitDepth,
        sample_format: SampleFormat,
    ) -> Option<Self> {
        let ty = match (bit_depth, sample_format) {
            (BitDepth::D16, SampleFormat::Singed) => SampleType::I16,
            (BitDepth::D24, SampleFormat::Singed) => SampleType::I24,
            (BitDepth::D32, SampleFormat::Singed) => SampleType::I32,
            (BitDepth::D32, SampleFormat::Floating) => SampleType::F32,
            _ => return None,
        };

        Some(Self {
            channels,
            freq_hz,
            sample_type: ty,
        })
    }
    /// Returns the amount of channels
    pub const fn channels(&self) -> ChannelCount {
        self.channels
    }

    /// Returns frequency in hz.
    ///
    /// amount of frames per seconds (a frame is Nsamples where N is the amount of channels).
    pub const fn freq(&self) -> u32 {
        self.freq_hz
    }

    /// Returns the amount of samples to be processed per second, in all channels.
    pub const fn samples_per_second(&self) -> u32 {
        self.freq_hz * self.channels as u32
    }

    /// Returns the amount of bits per sample. (aka audio bit depth)
    pub const fn bit_depth(&self) -> BitDepth {
        self.sample_type.bit_depth()
    }

    /// Returns the sample format.
    ///
    /// eg. floating point or integer
    pub const fn sample_format(&self) -> SampleFormat {
        self.sample_type.sample_format()
    }

    pub const fn sample_type(&self) -> SampleType {
        self.sample_type
    }
}
