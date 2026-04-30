use std::{
    io::{self, ErrorKind, Read, Seek, SeekFrom},
    sync::{Condvar, Mutex},
};

use lune_abi::misc::{AudioFormat, SampleFormat};

use crate::stream::Stream;

/// A higher level simple Audio player.
///
/// This audio player directly communicates with the server for operation, it doesn't support userspace mixing or whatever.
///
/// # Usage
/// Construct with [`Self::new`] or [`Self::load_wav`] and then play with [`Self::play`].
pub struct AudioPlayer<R: Read> {
    stream: Mutex<(R, Stream, usize)>,
    stream_paused: Mutex<bool>,
    awaiting_resume: Condvar,
    max_read_bytes: usize,
}

impl<R: Read + Seek> AudioPlayer<R> {
    /// Parses a WAV File and returns an AudioPlayer over it.
    pub fn load_wav(mut reader: R) -> io::Result<Self> {
        let (format, max) = parse_wav_file(&mut reader)?;
        println!("format: {format:#?}, max: {max}");
        Ok(Self::new(format, reader, max))
    }
}

impl<R: Read> AudioPlayer<R> {
    /// Constructs a new AudioPlayer.
    pub fn new(format: AudioFormat, reader: R, max_read_bytes: usize) -> Self {
        let max_samples = format.samples_per_second();
        let stream = Stream::create(format, max_samples);

        Self {
            stream: Mutex::new((reader, stream, 0)),
            stream_paused: Mutex::new(false),
            awaiting_resume: Condvar::new(),
            max_read_bytes,
        }
    }

    pub fn pause(&self) {
        *self.stream_paused.lock().unwrap() = true;
    }

    pub fn resume(&self) {
        self.awaiting_resume.notify_all();
    }

    /// Plays audio until reading fails, returns the amount of data read from the given reader.
    ///
    /// Audio playback can be stopped with [`Self::pause`] and resumed again [`Self::resume`], calling this would reserve the current thread for audio playback.
    pub fn play(&self) -> std::io::Result<usize> {
        let mut guard = self.stream.lock().expect("AudioPlayer lock broken!!1!");
        let (data, stream, read_counter) = &mut *guard;
        let m_bytes = self.max_read_bytes - *read_counter;

        let frames_sec = stream.format().freq() as usize;
        let samples_per_frame = stream.format().channels() as usize;

        let bpp_sample = stream.bytes_per_sample() as usize;
        let bpp_frame = samples_per_frame * bpp_sample;

        let m_frames = m_bytes / bpp_frame;

        let mut total_playing = 0;
        let mut pending_sync = 0;
        loop {
            let paused = self.stream_paused.lock().unwrap();
            if *paused {
                drop(
                    self.awaiting_resume
                        .wait_while(paused, |v| *v == true)
                        .expect("Failure to wait for audio playback to resume"),
                );
            } else {
                drop(paused);
            }

            let read_off = pending_sync * bpp_frame;
            let read_max = (m_bytes - (total_playing * bpp_frame))
                .min(bpp_frame * frames_sec.saturating_sub(pending_sync));
            let data_read = data.read(&mut stream.buf_mut()[read_off..read_off + read_max])?;
            *read_counter += data_read;

            pending_sync += data_read / bpp_frame;

            let synced = stream.sync(0, (pending_sync * samples_per_frame) as u32);

            let synced_frames = synced as usize / samples_per_frame;
            pending_sync -= synced_frames;
            total_playing += synced_frames;

            stream
                .buf_mut()
                .copy_within((synced_frames * bpp_frame).., 0);

            if total_playing >= m_frames {
                break;
            }
        }

        Ok(total_playing as usize * bpp_frame)
    }
}
#[repr(C)]
struct WavRiffDesc {
    // == RIFF
    chunk_id: [u8; 4],
    chunk_size: u32,
    // == WAVE
    format: [u8; 4],
}
// #[repr(C)]
// struct WavDataChunkH {
//     /// == data
//     subchunk_2_id: [u8; 4],
//     /** == NumSamples * NumChannels * BitsPerSample/8

//     This is the number of bytes in the data.
//     You can also think of this as the size of the read of the subchunk following this number.
//     */
//     subchunk_2_size: u32,
// }
#[repr(C)]
struct WavFmtChunk {
    audio_format: u16,
    num_channels: u16,
    sample_rate: u32,
    /// == SampleRate * NumChannels * BitsPerSample/8
    byte_rate: u32,
    /// == NumChannels * BitsPerSample/8
    block_align: u16,
    bits_per_sample: u16,
}

#[repr(C)]
struct RIFFChunkH {
    subchunk_id: [u8; 4],
    subchunk_size: u32,
}

fn parse_wav_file<R: Read + Seek>(reader: &mut R) -> std::io::Result<(AudioFormat, usize)> {
    let mut riff_desc = [0u8; size_of::<WavRiffDesc>()];
    reader.read_exact(&mut riff_desc)?;

    let riff_desc: WavRiffDesc = unsafe { core::mem::transmute(riff_desc) };
    if riff_desc.chunk_id != *b"RIFF" && riff_desc.format != *b"WAVE" {
        return Err(std::io::Error::new(
            std::io::ErrorKind::Other,
            "RIFF Header invalid",
        ));
    }

    fn next_chunk<R: Read>(reader: &mut R) -> std::io::Result<Option<RIFFChunkH>> {
        let mut ch_bytes = [0u8; size_of::<RIFFChunkH>()];

        if reader.read(&mut ch_bytes)? != ch_bytes.len() {
            return Ok(None);
        }

        Ok(Some(unsafe { core::mem::transmute(ch_bytes) }))
    }

    let mut data_info = None;
    let mut fmt_chunk: Option<WavFmtChunk> = None;

    while let Some(chunk) = next_chunk(reader)? {
        match &chunk.subchunk_id {
            b"fmt " => {
                let mut fmt_chunk_bytes = [0u8; size_of::<WavFmtChunk>()];
                reader.read_exact(&mut fmt_chunk_bytes)?;
                fmt_chunk = Some(unsafe { core::mem::transmute(fmt_chunk_bytes) });
            }
            b"data" => {
                data_info = Some((reader.stream_position()?, chunk.subchunk_size));
            }
            _ => {
                reader.seek_relative(chunk.subchunk_size as i64)?;
            }
        }
    }

    if let Some(fmt_c) = fmt_chunk
        && let Some((data_seek, data_size)) = data_info
    {
        macro_rules! unsupported {
            ($msg:literal) => {{ io::Error::new(ErrorKind::Unsupported, $msg) }};
        }

        let sample_kind = match fmt_c.audio_format {
            1 => SampleFormat::Singed,
            3 => SampleFormat::Floating,
            _ => return Err(unsupported!("Audio format data kind")),
        };
        reader.seek(SeekFrom::Start(data_seek))?;
        AudioFormat::from_raw(
            fmt_c.num_channels as u8,
            fmt_c.sample_rate,
            fmt_c.bits_per_sample as u8,
            sample_kind,
        )
        .ok_or_else(|| unsupported!("Audio format"))
        .map(|f| (f, data_size as usize))
    } else {
        Err(std::io::Error::new(
            std::io::ErrorKind::Other,
            "No fmt or data chunk",
        ))
    }
}
