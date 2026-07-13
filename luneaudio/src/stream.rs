use std::{
    collections::HashMap,
    fs::{File, OpenOptions},
    io::Write,
    sync::{Arc, Condvar, LazyLock, Mutex},
};

use libserver::log;
use lune_abi::StreamID;
pub use lune_abi::misc::*;
use safa_api::shm::SharedObject;

pub const TICK_DURATION_MS: usize = 25;
const BUF_PADDING_MUL: usize = 4;

#[derive(Debug)]
pub struct Mixer {
    format: AudioFormat,
    out_buffer: Box<[u8]>,
    write_ptr: usize,
    pending_buffer: Vec<f32>,
    samples_per_ms: usize,
    file: File,
    streams: Vec<Stream>,
    stream_ids: HashMap<StreamID, usize>,
    next_stream_id: StreamID,
    pending_samples: usize,
}

impl Mixer {
    /// Create a new Audio Mixer from system's audio drivers.
    pub fn create() -> Self {
        let audio_devices =
            std::fs::read_dir("dev:/audio").expect("Failed to retrieve audio devices");
        let device_path = audio_devices
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().unwrap().is_file())
            .map(|e| e.path())
            .next()
            .expect("No audio device found");

        log!("Using Audio driver: {}", device_path.display());
        let file = OpenOptions::new()
            .write(true)
            .open(device_path)
            .expect("Failed to open audio driver");

        /// Describes the PCM Format an Audio card accepts.
        #[repr(C)]
        #[derive(Debug, Clone, Copy, Default)]
        pub struct AudioInfo {
            freq_hz: u32,
            __padding: [u8; 2],
            bits_per_sample: u8,
            channels: u8,
        }

        const CMD_GET_AC_AUDIO_INFO: u16 = 0x1001;
        const CMD_GET_AC_BUF_SIZE: u16 = 0x1002;

        use std::os::safaos::io::IoUtils;
        let mut buf_size = 0usize;
        let mut audio_info = AudioInfo::default();

        file.send_command(CMD_GET_AC_BUF_SIZE, (&raw mut buf_size) as u64)
            .expect("Failed to get Audio Buffer Size");
        log!("Audio driver buffer size: {buf_size}");
        file.send_command(CMD_GET_AC_AUDIO_INFO, (&raw mut audio_info) as u64)
            .expect("Failed to get Audio Info");
        log!("Audio driver Audio Info: {audio_info:#?}");

        let audio_format = AudioFormat::from_raw(
            audio_info.channels,
            audio_info.freq_hz,
            audio_info.bits_per_sample,
            SampleFormat::Singed,
        )
        .expect("Invalid audio format to construct");

        let samples_per_ms = audio_format.samples_per_second() as usize / 1000;
        Self {
            format: audio_format,
            out_buffer: vec![
                0;
                samples_per_ms
                    * TICK_DURATION_MS
                    * BUF_PADDING_MUL
                    * (audio_format.bit_depth() as usize / 8)
            ]
            .into_boxed_slice(),
            write_ptr: 0,
            pending_buffer: vec![0.; samples_per_ms * TICK_DURATION_MS * BUF_PADDING_MUL],
            samples_per_ms,
            file,
            streams: Vec::new(),
            stream_ids: HashMap::new(),
            next_stream_id: 0,
            pending_samples: 0,
        }
    }

    fn ac_queued_samples(&self) -> usize {
        use std::os::safaos::io::IoUtils;

        const CMD_GET_AC_QUEUED_SAMPLES: u16 = 0x1003;
        let mut count = 0usize;
        self.file
            .send_command(CMD_GET_AC_QUEUED_SAMPLES, (&raw mut count) as u64)
            .expect("Failed to send a command to file");
        count
    }

    fn flush_existing(&mut self) -> usize {
        let bytes_wrote = self
            .file
            .write(&self.out_buffer[..self.write_ptr])
            .expect("Failed to write to audio driver");

        self.out_buffer.copy_within(bytes_wrote.., 0);
        self.write_ptr -= bytes_wrote;
        bytes_wrote
    }

    pub fn flush(&mut self) -> (usize, usize) {
        // If we don't do that flush could stall as stalling samples will never flush and buf.len() would be zero.
        let before = self.flush_existing();

        let ac_pending_samples = self.ac_queued_samples();
        let ava_samples = self.pending_samples;
        let buf = &mut self.out_buffer[self.write_ptr..];
        let bytes_per_sample = self.format.bit_depth() as usize / 8;

        let single_flush_max_ms = TICK_DURATION_MS * BUF_PADDING_MUL;
        let to_flush_ms = single_flush_max_ms.abs_diff(ac_pending_samples / self.samples_per_ms);
        let normalized_max = to_flush_ms * self.samples_per_ms;

        let samples = (buf.len() / bytes_per_sample)
            .min(normalized_max)
            .min(self.pending_buffer.len())
            .min(ava_samples);
        if samples == 0 {
            return (0, 0);
        }
        let drained = self.pending_buffer.drain(..samples);
        self.pending_samples -= samples;

        match self.format.bit_depth() {
            BitDepth::D16 => {
                for (i, sample) in drained.enumerate() {
                    let sample_i16 = (sample.clamp(-1., 1.) * i16::MAX as f32) as i16;
                    let sample_bytes = i16::to_le_bytes(sample_i16);

                    buf[i * 2] = sample_bytes[0];
                    buf[(i * 2) + 1] = sample_bytes[1];
                }
            }
            BitDepth::D24 => {
                for (i, sample) in drained.enumerate() {
                    let sample_i32 = (sample.clamp(-1., 1.) * (1 << 24) as f32) as i32;
                    let sample_bytes = i32::to_le_bytes(sample_i32);

                    buf[i * 3] = sample_bytes[0];
                    buf[(i * 3) + 1] = sample_bytes[1];
                    buf[(i * 3) + 2] = sample_bytes[2];
                }
            }
            BitDepth::D32 => {
                for (i, sample) in drained.enumerate() {
                    let sample_i32 = (sample.clamp(-1., 1.) * i32::MAX as f32) as i32;
                    let sample_bytes = i32::to_le_bytes(sample_i32);

                    buf[i * 4] = sample_bytes[0];
                    buf[(i * 4) + 1] = sample_bytes[1];
                    buf[(i * 4) + 2] = sample_bytes[2];
                    buf[(i * 4) + 3] = sample_bytes[3];
                }
            }
        }

        for stream in &mut self.streams {
            let removed_samples = stream.pending_samples.min(samples);
            stream.pending_samples -= removed_samples;
        }

        self.pending_buffer
            .resize(self.pending_buffer.capacity(), 0.);
        self.write_ptr += samples * bytes_per_sample;

        let wrote = self.flush_existing() + before;
        (
            wrote / bytes_per_sample,
            self.out_buffer.len().saturating_sub(self.write_ptr) / bytes_per_sample,
        )
    }

    pub fn set_volume_prec(&mut self, stream_id: StreamID, volume_prec: f32) -> Result<(), ()> {
        let index = self.stream_ids.get(&stream_id).ok_or(())?;
        Ok(self.streams[*index].volume = volume_prec.clamp(0., 500.) / 100.)
    }

    pub fn sync_stream(
        &mut self,
        stream_id: StreamID,
        sample_offset: usize,
        samples: usize,
    ) -> Result<usize, ()> {
        let index = self.stream_ids.get(&stream_id).ok_or(())?;
        let (processed, pending) = self.streams[*index].sync(
            samples,
            sample_offset,
            &self.format,
            &mut self.pending_buffer,
        );

        self.pending_samples = self.pending_samples.max(pending);
        Ok(processed)
    }

    #[inline]
    pub fn fix_id(&mut self) {
        while self.stream_ids.get(&self.next_stream_id).is_some() {
            self.next_stream_id += 1;
        }
    }

    pub fn add_stream(
        &mut self,
        in_object: Arc<SharedObject>,
        samples_size: usize,
        format: AudioFormat,
    ) -> Result<StreamID, ()> {
        let id = self.next_stream_id;

        let mut stream = Stream::create(in_object, samples_size, format, self.pending_samples)?;
        stream.stream_id = id;

        self.streams.push(stream);

        self.stream_ids.insert(id, self.streams.len() - 1);
        self.next_stream_id += 1;
        self.fix_id();
        Ok(id)
    }

    pub fn remove_stream(&mut self, stream_id: StreamID) -> bool {
        let Some(index) = self.stream_ids.remove(&stream_id) else {
            return false;
        };

        let old_stream = self.streams.swap_remove(index);
        let replacement = self.streams.get_mut(index);

        assert_eq!(old_stream.stream_id, stream_id);
        if let Some(r) = replacement {
            assert!(self.stream_ids.insert(r.stream_id, index).is_some());
        }
        self.next_stream_id = self.next_stream_id.min(stream_id);
        true
    }
}

#[derive(Debug)]
pub struct Stream {
    // Magic
    stream_id: StreamID,
    source_object: Arc<SharedObject>,
    // pending_buffer: Box<[f32]>,
    pending_samples: usize,
    // Configuration
    volume: f32,
    source_format: AudioFormat,
}

#[inline(always)]
fn resample<'a>(
    get_sample: impl Fn(usize) -> Option<f32>,
    src_samples_len: usize,
    dst_freq: u32,
    src_freq: u32,
    channels: usize,
    max_samples: usize,
    mut add_sample: impl FnMut(usize, f32),
) -> usize {
    if src_freq == dst_freq {
        let samples_count = src_samples_len.min(max_samples);

        for i in 0..samples_count {
            let sample = get_sample(i).expect("Invalid sample count given");
            add_sample(i, sample);
        }

        return samples_count;
    }

    let ratio = src_freq as f32 / dst_freq as f32;
    let dst_frame_len = ((src_samples_len / channels) as f32 / ratio).ceil() as usize;
    let frames_to_add = dst_frame_len.min(max_samples / channels);

    for i in 0..frames_to_add {
        let src_pos = i as f32 * ratio;
        let src_f_idx = src_pos as usize;
        let t = src_pos.fract();

        for ch in 0..channels {
            let Some(a) = get_sample(src_f_idx * channels + ch) else {
                continue;
            };
            let b = get_sample((src_f_idx + 1) * channels + ch).unwrap_or(a);

            add_sample(i * channels + ch, a + (b - a) * t);
        }
    }

    (frames_to_add as f32 * ratio) as usize * channels
}

#[inline]
fn ii16tf32(bytes: &[u8], idx: usize) -> Option<f32> {
    let s = idx * size_of::<i16>();
    let i = i16::from_le_bytes([*bytes.get(s)?, bytes[s + 1]]);
    Some((i as f32) / i16::MAX as f32)
}

#[inline]
fn ii32tf32(bytes: &[u8], idx: usize) -> Option<f32> {
    let s = idx * size_of::<i32>();
    let i = i32::from_le_bytes([*bytes.get(s)?, bytes[s + 1], bytes[s + 2], bytes[s + 3]]);
    Some((i as f32) / i32::MAX as f32)
}

#[inline]
fn if32tf32(bytes: &[u8], idx: usize) -> Option<f32> {
    let s = idx * size_of::<f32>();
    let i = f32::from_le_bytes([*bytes.get(s)?, bytes[s + 1], bytes[s + 2], bytes[s + 3]]);
    Some(i)
}

#[inline]
fn ii24tf32(bytes: &[u8], idx: usize) -> Option<f32> {
    let s = idx * 3;
    let i = i32::from_le_bytes([*bytes.get(s)?, bytes[s + 1], bytes[s + 2], 0]) << 8 >> 8;
    Some((i as f32) / (1 << 23) as f32)
}

macro_rules! prev_multiply_of {
    ($num:expr,$other:expr) => {{ ($num / $other) * $other }};
}

impl Stream {
    /// Create a new stream in the given object.
    fn create(
        in_object: Arc<SharedObject>,
        samples_size: usize,
        format: AudioFormat,
        debug: usize,
    ) -> Result<Self, ()> {
        if in_object.data_ptr().len() <= samples_size / (format.bit_depth() as usize / 8) {
            return Err(());
        }
        log!(
            "Create stream, samples size: {samples_size}, format: {format:#?}, debug value: {debug}"
        );
        Ok(Self {
            stream_id: 0,
            source_format: format,
            source_object: in_object,
            pending_samples: 0,
            volume: 1.,
        })
    }

    /// Syncs `samples` samples from the stream at `sample_offset` to the given mixer buffer `sync_to`.
    pub fn sync(
        &mut self,
        samples: usize,
        sample_offset: usize,
        dst_format: &AudioFormat,
        sync_to: &mut [f32],
    ) -> (usize, usize) {
        let format = self.source_format;
        let bytes_per_sample = format.bit_depth() as usize / 8;

        let source_buffer = unsafe { self.source_object.data_ptr().as_ref() };

        let samples_to_sync = prev_multiply_of!(
            (source_buffer.len() / bytes_per_sample)
                .saturating_sub(sample_offset)
                .min(samples),
            format.channels() as usize
        );
        let max_sample_usage = prev_multiply_of!(
            (sync_to.len().saturating_sub(self.pending_samples) as f32
                / (dst_format.channels() as u8 as f32 / format.channels() as u8 as f32))
                as usize,
            dst_format.channels() as usize
        );

        // libserver::dlog!(
        //     "sync to len: {}, max usage: {}, pending: {}, samples to sync: {samples_to_sync}, off: {sample_offset}, requested: {samples}",
        //     sync_to.len(),
        //     max_sample_usage,
        //     self.pending_samples
        // );

        let idx_offset = sample_offset * bytes_per_sample;
        let src_buf = &source_buffer[idx_offset..idx_offset + (samples_to_sync * bytes_per_sample)];

        let wr_off = self.pending_samples;
        let mut wrote = 0;

        let processed_samples = resample(
            |idx| match format.sample_type() {
                // the i[T]tf32 functions, indexes into the given source buffer in samples assuming a type and converts.
                SampleType::I16 => ii16tf32(src_buf, idx),
                SampleType::I24 => ii24tf32(src_buf, idx),
                SampleType::F32 => if32tf32(src_buf, idx),
                SampleType::I32 => ii32tf32(src_buf, idx),
            },
            src_buf.len() / bytes_per_sample,
            dst_format.freq(),
            format.freq(),
            format.channels() as usize,
            max_sample_usage,
            |i, sample| match (format.channels(), dst_format.channels()) {
                (ChannelCount::Single, ChannelCount::Dual) => {
                    sync_to[wr_off + (i * 2)] += sample * self.volume;
                    let Some(b) = sync_to.get_mut(wr_off + (i * 2) + 1) else {
                        wrote += 1;
                        return;
                    };
                    *b += sample * self.volume;
                    wrote += 2;
                }
                (ChannelCount::Dual, ChannelCount::Single) => {
                    sync_to[wr_off + (i / 2)] += (sample * self.volume) / 2.;
                    if i % 2 == 0 {
                        wrote += 1;
                    }
                }
                (ChannelCount::Single, ChannelCount::Single)
                | (ChannelCount::Dual, ChannelCount::Dual) => {
                    sync_to[wr_off + i] += sample * self.volume;
                    wrote += 1;
                }
            },
        );

        self.pending_samples += wrote;
        (processed_samples, self.pending_samples)
    }
}

/// Global Audio Mixer
pub static MIXER: LazyLock<Mutex<Mixer>> = LazyLock::new(|| Mutex::new(Mixer::create()));
static AUDIO_WAITERS: Condvar = Condvar::new();

/// Returns information about the mixer.
pub fn mixer_audio_info() -> AudioFormat {
    MIXER.lock().unwrap().format
}

/// Attempts to flush all of the pending audio samples, may not block.
pub fn try_flush_pending() -> bool {
    let mut guard = MIXER.lock().expect("Flush pending failed");
    if guard.pending_samples == 0 {
        return false;
    }

    let (flushed, _left) = guard.flush();
    flushed != 0
}

/// attempts to flush all of the pending audio samples to the audio driver, from the mixer.
///
/// returns the amount of samples flushed and the amount of free samples in the pending buffer.
///
/// May block until there are pending samples.
pub fn flush_pending() -> (usize, usize) {
    let mut guard = MIXER.lock().expect("Flush pending failed");
    // let mut guard = AUDIO_WAITERS
    //     .wait_while(guard, |guard| guard.pending_samples == 0)
    //     .expect("Failed to wait for pending samples to not be zero");
    let (flushed, left) = guard.flush();
    (flushed, left)
}

/// Creates a new stream using the given `in_object` shared memory region.
///
/// returns an Err(()) if the given object cannot be used for creating the stream, i.e being too small.
pub fn create_stream(
    in_object: Arc<SharedObject>,
    format: AudioFormat,
    samples_count: usize,
) -> Result<StreamID, ()> {
    MIXER
        .lock()
        .expect("Creating stream failed (acquiring lock)")
        .add_stream(in_object, samples_count, format)
}

pub fn remove_stream(stream_id: StreamID) -> bool {
    MIXER
        .lock()
        .expect("Removing stream failed (acquiring lock)")
        .remove_stream(stream_id)
}

/// Syncs the given stream's memory `stream_id`  from the given offset `offset` in samples.
///
/// Syncs `samples` samples at most, returns an error if no such stream exists, or Ok(n) where n is the amount of synced samples.
pub fn sync_stream(stream_id: StreamID, offset: usize, samples: usize) -> Result<usize, ()> {
    let synced = MIXER
        .lock()
        .expect("Sync stream failed (acquiring lock)")
        .sync_stream(stream_id, offset, samples)?;
    if synced != 0 {
        AUDIO_WAITERS.notify_one();
    }

    Ok(synced)
}
