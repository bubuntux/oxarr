use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

/// Canonical metadata extracted from a media filename or release title.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct ParsedMediaInfo {
    pub title: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub year: Option<u16>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub season: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub episode: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resolution: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provider: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub video_codec: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub audio_codec: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub audio_channels: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub release_group: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub edition: Option<String>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub tags: BTreeMap<String, String>,
}

/// Backwards compatibility alias.
pub type ParsedReleaseInfo = ParsedMediaInfo;

/// Parses a media filename or release title into structured metadata.
pub fn parse(filename: &str) -> ParsedMediaInfo {
    let parsed = parse_torrent_title::parse(filename);

    let title = parsed.title().trim().to_owned();
    let year = parsed.year().and_then(|v| {
        v.as_int()
            .and_then(|i| u16::try_from(i).ok())
            .or_else(|| v.as_str().and_then(|s| s.parse::<u16>().ok()))
    });
    let season = parsed.season().and_then(|s| u32::try_from(s).ok());
    let episode = parsed.episode().and_then(|e| u32::try_from(e).ok());
    let resolution = parsed.resolution().map(|s| s.to_string());
    let source = parsed.source().map(|s| s.to_string());
    let video_codec = parsed.codec().map(|s| s.to_string());
    let audio_codec = parsed.audio().map(|s| s.to_string());
    let audio_channels = parsed.audio_channels().map(|s| s.to_string());
    let release_group = parsed.group().map(|s| s.to_string());

    let mut tags = BTreeMap::new();
    if let Some(network) = parsed.get("network").and_then(|v| v.as_str()) {
        tags.insert("provider".to_owned(), network.to_string());
    }

    let provider = tags.get("provider").cloned();

    ParsedMediaInfo {
        title,
        year,
        season,
        episode,
        resolution,
        source,
        provider,
        video_codec,
        audio_codec,
        audio_channels,
        release_group,
        edition: None,
        tags,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_movie_release() {
        let info = parse("The.Matrix.1999.2160p.UHD.BluRay.x265-FraMeSToR.mkv");
        assert_eq!(info.title, "The Matrix");
        assert_eq!(info.year, Some(1999));
        assert_eq!(info.resolution.as_deref(), Some("4k"));
        assert_eq!(info.video_codec.as_deref(), Some("x265"));
        assert_eq!(info.release_group.as_deref(), Some("FraMeSToR"));
    }

    #[test]
    fn parses_tv_show_episode() {
        let info = parse("Breaking.Bad.S01E01.1080p.BluRay.DTS.x264-CtrlHD.mkv");
        assert_eq!(info.title, "Breaking Bad");
        assert_eq!(info.season, Some(1));
        assert_eq!(info.episode, Some(1));
        assert_eq!(info.resolution.as_deref(), Some("1080p"));
        assert_eq!(info.release_group.as_deref(), Some("CtrlHD"));
    }
}
