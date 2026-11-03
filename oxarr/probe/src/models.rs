use std::collections::HashMap;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FfprobeOutput {
    #[serde(default)]
    pub streams: Vec<Stream>,
    pub format: Option<Format>,
    #[serde(default)]
    pub chapters: Vec<Chapter>,
}

impl FfprobeOutput {
    pub fn primary_video(&self) -> Option<&Stream> {
        self.streams.iter().find(|s| s.is_video())
    }

    pub fn primary_audio(&self) -> Option<&Stream> {
        self.streams.iter().find(|s| s.is_audio())
    }

    pub fn video_streams(&self) -> impl Iterator<Item = &Stream> {
        self.streams.iter().filter(|s| s.is_video())
    }

    pub fn audio_streams(&self) -> impl Iterator<Item = &Stream> {
        self.streams.iter().filter(|s| s.is_audio())
    }

    pub fn subtitle_streams(&self) -> impl Iterator<Item = &Stream> {
        self.streams.iter().filter(|s| s.is_subtitle())
    }

    pub fn duration_seconds(&self) -> Option<f64> {
        self.format
            .as_ref()
            .and_then(|f| f.duration_seconds())
            .or_else(|| self.primary_video().and_then(|s| s.duration_seconds()))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum StreamType {
    Video,
    Audio,
    Subtitle,
    Data,
    Attachment,
    #[serde(other)]
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Stream {
    pub index: u32,
    pub codec_name: Option<String>,
    pub codec_long_name: Option<String>,
    pub profile: Option<String>,
    pub codec_type: Option<StreamType>,
    pub codec_tag_string: Option<String>,
    pub codec_tag: Option<String>,

    pub width: Option<u32>,
    pub height: Option<u32>,
    pub coded_width: Option<u32>,
    pub coded_height: Option<u32>,
    pub sample_aspect_ratio: Option<String>,
    pub display_aspect_ratio: Option<String>,
    pub pix_fmt: Option<String>,
    pub level: Option<i32>,
    pub color_range: Option<String>,
    pub color_space: Option<String>,
    pub color_transfer: Option<String>,
    pub color_primaries: Option<String>,

    pub r_frame_rate: Option<String>,
    pub avg_frame_rate: Option<String>,
    pub time_base: Option<String>,
    pub start_pts: Option<i64>,
    pub start_time: Option<String>,
    pub duration_ts: Option<i64>,
    pub duration: Option<String>,
    pub bit_rate: Option<String>,
    pub max_bit_rate: Option<String>,
    pub bits_per_raw_sample: Option<String>,
    pub nb_frames: Option<String>,

    // Audio-specific
    pub sample_fmt: Option<String>,
    pub sample_rate: Option<String>,
    pub channels: Option<u32>,
    pub channel_layout: Option<String>,
    pub bits_per_sample: Option<u32>,

    pub disposition: Option<StreamDisposition>,
    #[serde(default)]
    pub tags: HashMap<String, String>,
}

impl Stream {
    pub fn is_video(&self) -> bool {
        matches!(self.codec_type, Some(StreamType::Video))
    }

    pub fn is_audio(&self) -> bool {
        matches!(self.codec_type, Some(StreamType::Audio))
    }

    pub fn is_subtitle(&self) -> bool {
        matches!(self.codec_type, Some(StreamType::Subtitle))
    }

    pub fn frame_rate(&self) -> Option<f64> {
        self.avg_frame_rate
            .as_deref()
            .and_then(parse_ratio_or_float)
            .or_else(|| {
                self.r_frame_rate
                    .as_deref()
                    .and_then(parse_ratio_or_float)
            })
    }

    pub fn duration_seconds(&self) -> Option<f64> {
        self.duration.as_deref().and_then(|d| d.parse::<f64>().ok())
    }

    pub fn bit_rate_bps(&self) -> Option<u64> {
        self.bit_rate.as_deref().and_then(|b| b.parse::<u64>().ok())
    }

    pub fn sample_rate_hz(&self) -> Option<u32> {
        self.sample_rate.as_deref().and_then(|s| s.parse::<u32>().ok())
    }

    pub fn language(&self) -> Option<&str> {
        self.tags
            .get("language")
            .or_else(|| self.tags.get("LANGUAGE"))
            .map(|s| s.as_str())
    }

    pub fn title(&self) -> Option<&str> {
        self.tags
            .get("title")
            .or_else(|| self.tags.get("TITLE"))
            .map(|s| s.as_str())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Format {
    pub filename: Option<String>,
    pub nb_streams: Option<u32>,
    pub nb_programs: Option<u32>,
    pub format_name: Option<String>,
    pub format_long_name: Option<String>,
    pub start_time: Option<String>,
    pub duration: Option<String>,
    pub size: Option<String>,
    pub bit_rate: Option<String>,
    pub probe_score: Option<i32>,
    #[serde(default)]
    pub tags: HashMap<String, String>,
}

impl Format {
    pub fn duration_seconds(&self) -> Option<f64> {
        self.duration.as_deref().and_then(|d| d.parse::<f64>().ok())
    }

    pub fn size_bytes(&self) -> Option<u64> {
        self.size.as_deref().and_then(|s| s.parse::<u64>().ok())
    }

    pub fn bit_rate_bps(&self) -> Option<u64> {
        self.bit_rate.as_deref().and_then(|b| b.parse::<u64>().ok())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct StreamDisposition {
    #[serde(default)]
    pub default: i32,
    #[serde(default)]
    pub dub: i32,
    #[serde(default)]
    pub original: i32,
    #[serde(default)]
    pub comment: i32,
    #[serde(default)]
    pub lyrics: i32,
    #[serde(default)]
    pub karaoke: i32,
    #[serde(default)]
    pub forced: i32,
    #[serde(default)]
    pub hearing_impaired: i32,
    #[serde(default)]
    pub visual_impaired: i32,
    #[serde(default)]
    pub clean_effects: i32,
    #[serde(default)]
    pub attached_pic: i32,
    #[serde(default)]
    pub timed_thumbnails: i32,
    #[serde(default)]
    pub captions: i32,
    #[serde(default)]
    pub descriptions: i32,
    #[serde(default)]
    pub metadata: i32,
    #[serde(default)]
    pub dependent: i32,
    #[serde(default)]
    pub still_image: i32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Chapter {
    pub id: i64,
    pub time_base: Option<String>,
    pub start: i64,
    pub start_time: Option<String>,
    pub end: i64,
    pub end_time: Option<String>,
    #[serde(default)]
    pub tags: HashMap<String, String>,
}

fn parse_ratio_or_float(val: &str) -> Option<f64> {
    if let Some((num_str, den_str)) = val.split_once('/') {
        let num: f64 = num_str.trim().parse().ok()?;
        let den: f64 = den_str.trim().parse().ok()?;
        if den != 0.0 {
            Some(num / den)
        } else {
            None
        }
    } else {
        val.trim().parse::<f64>().ok()
    }
}
