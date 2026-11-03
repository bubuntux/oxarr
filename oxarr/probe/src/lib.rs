pub mod error;
pub mod models;
pub mod runner;

pub use error::ProbeError;
pub use models::{
    Chapter, FfprobeOutput, Format, Stream, StreamDisposition, StreamType,
};
pub use runner::{
    parse, parse_str, probe_sync, probe_sync_with_config, FfprobeConfig,
};

#[cfg(feature = "tokio")]
pub use runner::{probe, probe_with_config};

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use super::*;

    const SAMPLE_JSON: &str = r#"{
        "streams": [
            {
                "index": 0,
                "codec_name": "hevc",
                "codec_long_name": "High Efficiency Video Coding",
                "profile": "Main 10",
                "codec_type": "video",
                "codec_tag_string": "hev1",
                "codec_tag": "0x31766568",
                "width": 3840,
                "height": 2160,
                "coded_width": 3840,
                "coded_height": 2160,
                "sample_aspect_ratio": "1:1",
                "display_aspect_ratio": "16:9",
                "pix_fmt": "yuv420p10le",
                "level": 150,
                "color_range": "tv",
                "color_space": "bt2020nc",
                "color_transfer": "smpte2084",
                "color_primaries": "bt2020",
                "r_frame_rate": "24/1",
                "avg_frame_rate": "24/1",
                "time_base": "1/24000",
                "duration": "7200.000000",
                "bit_rate": "25000000",
                "disposition": {
                    "default": 1,
                    "forced": 0
                },
                "tags": {
                    "language": "eng",
                    "title": "4K HDR"
                }
            },
            {
                "index": 1,
                "codec_name": "eac3",
                "codec_long_name": "ATSC A/52B (AC-3, E-AC-3)",
                "codec_type": "audio",
                "channels": 6,
                "channel_layout": "5.1(side)",
                "sample_rate": "48000",
                "bit_rate": "768000",
                "disposition": {
                    "default": 1
                },
                "tags": {
                    "language": "eng"
                }
            },
            {
                "index": 2,
                "codec_name": "subrip",
                "codec_type": "subtitle",
                "disposition": {
                    "forced": 1
                },
                "tags": {
                    "language": "spa"
                }
            }
        ],
        "format": {
            "filename": "movie.mkv",
            "nb_streams": 3,
            "format_name": "matroska,webm",
            "format_long_name": "Matroska / WebM",
            "duration": "7200.000000",
            "size": "22500000000",
            "bit_rate": "25000000",
            "probe_score": 100,
            "tags": {
                "encoder": "libebml"
            }
        },
        "chapters": [
            {
                "id": 0,
                "time_base": "1/1000",
                "start": 0,
                "start_time": "0.000000",
                "end": 300000,
                "end_time": "300.000000",
                "tags": {
                    "title": "Prologue"
                }
            }
        ]
    }"#;

    #[test]
    fn parses_ffprobe_json() {
        let output = parse_str(SAMPLE_JSON).expect("should parse valid ffprobe json");

        assert_eq!(output.streams.len(), 3);
        assert_eq!(output.chapters.len(), 1);

        let video = output.primary_video().expect("should find primary video");
        assert_eq!(video.codec_name.as_deref(), Some("hevc"));
        assert_eq!(video.width, Some(3840));
        assert_eq!(video.height, Some(2160));
        assert_eq!(video.frame_rate(), Some(24.0));
        assert_eq!(video.language(), Some("eng"));
        assert_eq!(video.title(), Some("4K HDR"));

        let audio = output.primary_audio().expect("should find primary audio");
        assert_eq!(audio.codec_name.as_deref(), Some("eac3"));
        assert_eq!(audio.channels, Some(6));
        assert_eq!(audio.sample_rate_hz(), Some(48000));
        assert_eq!(audio.bit_rate_bps(), Some(768000));

        let subs: Vec<_> = output.subtitle_streams().collect();
        assert_eq!(subs.len(), 1);
        assert_eq!(subs[0].codec_name.as_deref(), Some("subrip"));
        assert_eq!(subs[0].language(), Some("spa"));
        assert_eq!(subs[0].disposition.as_ref().map(|d| d.forced), Some(1));

        let format = output.format.expect("should have format");
        assert_eq!(format.format_name.as_deref(), Some("matroska,webm"));
        assert_eq!(format.duration_seconds(), Some(7200.0));
        assert_eq!(format.size_bytes(), Some(22500000000));

        assert_eq!(output.chapters[0].tags.get("title").map(|s| s.as_str()), Some("Prologue"));
    }

    #[test]
    fn parses_fractional_frame_rates() {
        let stream = Stream {
            index: 0,
            codec_name: None,
            codec_long_name: None,
            profile: None,
            codec_type: Some(StreamType::Video),
            codec_tag_string: None,
            codec_tag: None,
            width: None,
            height: None,
            coded_width: None,
            coded_height: None,
            sample_aspect_ratio: None,
            display_aspect_ratio: None,
            pix_fmt: None,
            level: None,
            color_range: None,
            color_space: None,
            color_transfer: None,
            color_primaries: None,
            r_frame_rate: None,
            avg_frame_rate: Some("24000/1001".to_string()),
            time_base: None,
            start_pts: None,
            start_time: None,
            duration_ts: None,
            duration: None,
            bit_rate: None,
            max_bit_rate: None,
            bits_per_raw_sample: None,
            nb_frames: None,
            sample_fmt: None,
            sample_rate: None,
            channels: None,
            channel_layout: None,
            bits_per_sample: None,
            disposition: None,
            tags: HashMap::new(),
        };

        let fps = stream.frame_rate().expect("should parse fps");
        assert!((fps - 23.976).abs() < 0.001);
    }
}
