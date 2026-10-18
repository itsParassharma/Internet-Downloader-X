use super::*;
fn hls(text: &str) -> Result<Plan, String> {
    plan(
        "https://media.example/v/main.m3u8",
        &StreamOptions {
            kind: "hls".into(),
            max_height: 0,
        },
        &mut |_| Ok(("https://media.example/v/main.m3u8".into(), text.into())),
    )
}
#[test]
fn hls_resolves_relative_urls_and_vod_segments() {
    let p = hls("#EXTM3U\n#EXTINF:1,\na.ts\n#EXTINF:1,\n../b.ts?token=1\n#EXT-X-ENDLIST").unwrap();
    assert_eq!(p.tracks[0].segments.len(), 2);
    assert_eq!(
        p.tracks[0].segments[1].url,
        "https://media.example/b.ts?token=1"
    );
}
#[test]
fn hls_explicit_and_implicit_byte_ranges() {
    let p=hls("#EXTM3U\n#EXT-X-MAP:URI=\"video.mp4\",BYTERANGE=\"100@0\"\n#EXT-X-BYTERANGE:20@100\nvideo.mp4\n#EXT-X-BYTERANGE:30\nvideo.mp4\n#EXT-X-ENDLIST").unwrap();
    assert_eq!(
        p.tracks[0]
            .segments
            .iter()
            .map(|s| s.range)
            .collect::<Vec<_>>(),
        vec![Some((0, 99)), Some((100, 119)), Some((120, 149))]
    );
    assert!(hls("#EXTM3U\n#EXT-X-BYTERANGE:20\nx.ts\n#EXT-X-ENDLIST").is_err());
}
#[test]
fn hls_selects_requested_quality_and_default_audio() {
    let mut fetched = vec![];
    let p = plan(
        "https://m.example/master.m3u8",
        &StreamOptions {
            kind: "hls".into(),
            max_height: 720,
        },
        &mut |u| {
            fetched.push(u.to_owned());
            Ok((
                u.into(),
                if u.ends_with("master.m3u8") {
                    r#"#EXTM3U
#EXT-X-MEDIA:TYPE=AUDIO,GROUP-ID="sound",DEFAULT=YES,URI="sound.m3u8",NAME="English, stereo"
#EXT-X-STREAM-INF:BANDWIDTH=2000,RESOLUTION=1920x1080,AUDIO="sound"
high.m3u8
#EXT-X-STREAM-INF:BANDWIDTH=1000,RESOLUTION=1280x720,AUDIO="sound"
low.m3u8"#
                        .into()
                } else {
                    "#EXTM3U\n#EXTINF:1,\nsegment.ts\n#EXT-X-ENDLIST".into()
                },
            ))
        },
    )
    .unwrap();
    assert_eq!(p.tracks.len(), 2);
    assert!(fetched[1].ends_with("low.m3u8"));
    assert!(fetched[2].ends_with("sound.m3u8"));
}
#[test]
fn hls_aes_sequence_iv_rotation_and_none() {
    let p=hls("#EXTM3U\n#EXT-X-MEDIA-SEQUENCE:42\n#EXT-X-KEY:METHOD=AES-128,URI=\"key\"\na.ts\n#EXT-X-KEY:METHOD=NONE\nb.ts\n#EXT-X-ENDLIST").unwrap();
    assert_eq!(
        p.tracks[0].segments[0].encryption.as_ref().unwrap().iv,
        42u128.to_be_bytes()
    );
    assert!(p.tracks[0].segments[1].encryption.is_none());
}
#[test]
fn hls_rejects_live_drm_discontinuities_and_local_files() {
    for text in [
        "#EXTM3U\na.ts",
        "#EXTM3U\n#EXT-X-KEY:METHOD=SAMPLE-AES,URI=\"key\"\na.ts\n#EXT-X-ENDLIST",
        "#EXTM3U\n#EXT-X-DISCONTINUITY\na.ts\n#EXT-X-ENDLIST",
        "#EXTM3U\nfile:///secret\n#EXT-X-ENDLIST",
    ] {
        assert!(hls(text).is_err());
    }
}
fn mpd(inner: &str) -> String {
    format!(
        r#"<MPD xmlns="urn:mpeg:dash:schema:mpd:2011" type="static" mediaPresentationDuration="PT4S"><Period>{inner}</Period></MPD>"#
    )
}
#[test]
fn dash_template_inheritance_numbers_and_separate_audio() {
    let text = mpd(
        r#"<BaseURL>media/</BaseURL><AdaptationSet mimeType="video/mp4"><SegmentTemplate timescale="1000" duration="1000" initialization="$RepresentationID$/init.mp4" media="$RepresentationID$/$Number%03d$.m4s" startNumber="7"/><Representation id="v" bandwidth="1000" height="720"/></AdaptationSet><AdaptationSet contentType="audio"><Representation id="a"><BaseURL>audio.mp4</BaseURL><SegmentBase/></Representation></AdaptationSet>"#,
    );
    let p = dash(&text, "https://m.example/manifest.mpd", 0).unwrap();
    assert_eq!(p.tracks.len(), 2);
    assert_eq!(p.tracks[0].segments.len(), 5);
    assert_eq!(
        p.tracks[0].segments[1].url,
        "https://m.example/media/v/007.m4s"
    );
    assert_eq!(p.tracks[1].kind, "audio");
}
#[test]
fn dash_timeline_negative_repeat_is_bounded() {
    let text = mpd(
        r#"<AdaptationSet contentType="video"><Representation id="v"><SegmentTemplate timescale="10" initialization="init.mp4" media="$Time$.m4s"><SegmentTimeline><S t="0" d="10" r="-1"/></SegmentTimeline></SegmentTemplate></Representation></AdaptationSet>"#,
    );
    let p = dash(&text, "https://m.example/main.mpd", 0).unwrap();
    assert_eq!(p.tracks[0].segments.len(), 5);
    assert!(p.tracks[0].segments[4].url.ends_with("30.m4s"));
}
#[test]
fn dash_timeline_next_time_and_presentation_offset() {
    let text = mpd(
        r#"<AdaptationSet contentType="video"><Representation id="v"><SegmentTemplate timescale="10" presentationTimeOffset="100" media="$Time$.m4s"><SegmentTimeline><S t="100" d="10" r="-1"/><S t="120" d="20"/></SegmentTimeline></SegmentTemplate></Representation></AdaptationSet>"#,
    );
    let p = dash(&text, "https://m.example/main.mpd", 0).unwrap();
    assert_eq!(p.tracks[0].segments.len(), 3);
    assert!(p.tracks[0].segments[2].url.ends_with("120.m4s"));
}
#[test]
fn dash_segment_list_ranges() {
    let text = mpd(
        r#"<AdaptationSet mimeType="audio/mp4"><Representation id="a"><BaseURL>a.mp4</BaseURL><SegmentList><Initialization range="0-99"/><SegmentURL mediaRange="100-199"/></SegmentList></Representation></AdaptationSet>"#,
    );
    let p = dash(&text, "https://m.example/main.mpd", 0).unwrap();
    assert_eq!(p.tracks[0].segments[1].range, Some((100, 199)));
}
#[test]
fn dash_rejects_drm_live_multi_period_and_zero_duration() {
    for text in [
        mpd("<ContentProtection/>"),
        mpd("").replace("static", "dynamic"),
        mpd("").replace("</Period>", "</Period><Period/>"),
        mpd(
            r#"<AdaptationSet contentType="video"><Representation><SegmentTemplate duration="0" media="x.m4s"/></Representation></AdaptationSet>"#,
        ),
    ] {
        assert!(dash(&text, "https://m.example/a.mpd", 0).is_err());
    }
}
#[test]
fn dash_rejects_xml_entities_and_excessive_expansion() {
    assert!(
        dash(
            "<!DOCTYPE MPD [<!ENTITY x SYSTEM 'file:///private'>]><MPD>&x;</MPD>",
            "https://m.example/a.mpd",
            0
        )
        .is_err()
    );
    let text = mpd(
        r#"<AdaptationSet contentType="video"><Representation><SegmentTemplate timescale="1000000" duration="1" media="x.m4s"/></Representation></AdaptationSet>"#,
    );
    assert!(dash(&text, "https://m.example/a.mpd", 0).is_err());
}
#[test]
fn templates_escape_dollars_and_reject_unknown_tokens() {
    assert_eq!(
        expanded("$$-$RepresentationID$-$Number%04d$", "vid", 1, 7, 0).unwrap(),
        "$-vid-0007"
    );
    assert!(expanded("$Number%099999d$", "v", 0, 0, 0).is_err());
    assert!(expanded("$Unknown$", "v", 0, 0, 0).is_err());
}
