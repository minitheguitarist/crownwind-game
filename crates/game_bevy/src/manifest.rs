//! Animation manifest loading (`docs/contracts/animation-manifest.schema.json`).
//!
//! The prince manifest is embedded at compile time from
//! `content/characters/prince_debug.json`, so frame indices always come from
//! the content file and never from hard-coded guesses. A tiny JSON reader is
//! used instead of adding a dependency (the lockfile is read-only for this
//! integration task).

use std::collections::BTreeMap;

/// Embedded prince manifest (single source of truth for frames and timing).
pub const PRINCE_MANIFEST_JSON: &str =
    include_str!("../../../content/characters/prince_debug.json");

/// Atlas image the manifest describes (asset path relative to `assets/`).
pub const PRINCE_ATLAS_PATH: &str = "characters/prince_debug.png";
/// Number of cells packed left to right in the atlas (448 / 32).
pub const PRINCE_ATLAS_COLUMNS: u32 = 14;

/// M3 playable prince (PixelLab animations, 96×96 frames, contact-line pivot).
pub const HERO_MANIFEST_JSON: &str = include_str!("../../../content/characters/prince_m3.json");
pub const HERO_ATLAS_PATH: &str = "characters/prince_m3.png";
/// 2592 / 96.
pub const HERO_ATLAS_COLUMNS: u32 = 27;

/// M3 princess, standing at the goal (PixelLab animations, 96×96 frames).
pub const PRINCESS_MANIFEST_JSON: &str =
    include_str!("../../../content/characters/princess_m3.json");
pub const PRINCESS_ATLAS_PATH: &str = "characters/princess_m3.png";
/// 2496 / 96.
pub const PRINCESS_ATLAS_COLUMNS: u32 = 26;

#[derive(Debug, Clone, PartialEq)]
pub struct Clip {
    pub frames: Vec<usize>,
    pub fps: f32,
    pub looping: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AnimationManifest {
    pub character_id: String,
    pub frame_width: u32,
    pub frame_height: u32,
    pub pivot: (f32, f32),
    pub clips: BTreeMap<String, Clip>,
}

impl AnimationManifest {
    pub fn clip(&self, name: &str) -> Option<&Clip> {
        self.clips.get(name)
    }

    /// Check the manifest against the atlas grid it will index into.
    pub fn validate(&self, atlas_columns: u32) -> Result<(), String> {
        if self.frame_width == 0 || self.frame_height == 0 {
            return Err("frame size must be positive".into());
        }
        for (name, clip) in &self.clips {
            if clip.frames.is_empty() {
                return Err(format!("clip `{name}` has no frames"));
            }
            if !(clip.fps > 0.0 && clip.fps <= 60.0) {
                return Err(format!("clip `{name}` has invalid fps {}", clip.fps));
            }
            if let Some(f) = clip.frames.iter().find(|&&f| f >= atlas_columns as usize) {
                return Err(format!("clip `{name}` frame {f} is outside the atlas"));
            }
        }
        let (px, py) = self.pivot;
        if px < 0.0 || py < 0.0 || px > self.frame_width as f32 || py > self.frame_height as f32 {
            return Err("pivot outside the frame".into());
        }
        Ok(())
    }

    pub fn parse(text: &str) -> Result<Self, String> {
        let value = Json::parse(text)?;
        let obj = value.as_object().ok_or("manifest must be an object")?;
        let get = |k: &str| obj.get(k).ok_or(format!("missing `{k}`"));
        if get("schema_version")?.as_f64() != Some(1.0) {
            return Err("unsupported schema_version".into());
        }
        let num = |k: &str| -> Result<f64, String> {
            get(k)?.as_f64().ok_or(format!("`{k}` must be a number"))
        };
        let pivot = match obj.get("pivot") {
            Some(p) => {
                let p = p.as_object().ok_or("pivot must be an object")?;
                let c = |k: &str| {
                    p.get(k)
                        .and_then(Json::as_f64)
                        .ok_or(format!("pivot.{k} must be a number"))
                };
                (c("x")? as f32, c("y")? as f32)
            }
            None => (0.0, 0.0),
        };
        let mut clips = BTreeMap::new();
        let anims = get("animations")?
            .as_object()
            .ok_or("animations must be an object")?;
        for (name, clip) in anims {
            let c = clip
                .as_object()
                .ok_or(format!("clip `{name}` must be an object"))?;
            let frames = c
                .get("frames")
                .and_then(Json::as_array)
                .ok_or(format!("clip `{name}` needs frames"))?
                .iter()
                .map(|f| {
                    f.as_f64()
                        .filter(|v| *v >= 0.0 && v.fract() == 0.0)
                        .map(|v| v as usize)
                        .ok_or(format!("clip `{name}` has a bad frame index"))
                })
                .collect::<Result<Vec<_>, _>>()?;
            let fps = c
                .get("fps")
                .and_then(Json::as_f64)
                .ok_or(format!("clip `{name}` needs fps"))? as f32;
            let looping = c
                .get("loop")
                .and_then(Json::as_bool)
                .ok_or(format!("clip `{name}` needs loop"))?;
            clips.insert(
                name.clone(),
                Clip {
                    frames,
                    fps,
                    looping,
                },
            );
        }
        Ok(Self {
            character_id: get("character_id")?
                .as_str()
                .ok_or("character_id must be a string")?
                .to_string(),
            frame_width: num("frame_width")? as u32,
            frame_height: num("frame_height")? as u32,
            pivot,
            clips,
        })
    }

    /// The embedded prince manifest. Panics only if the content file is
    /// malformed, which the unit tests catch at build/test time.
    pub fn prince() -> Self {
        let m = Self::parse(PRINCE_MANIFEST_JSON).expect("prince manifest must parse");
        m.validate(PRINCE_ATLAS_COLUMNS)
            .expect("prince manifest must match the atlas");
        m
    }

    /// The embedded M3 playable prince.
    pub fn hero() -> Self {
        Self::embedded(HERO_MANIFEST_JSON, HERO_ATLAS_COLUMNS)
    }

    /// The embedded M3 princess (goal character).
    pub fn princess() -> Self {
        Self::embedded(PRINCESS_MANIFEST_JSON, PRINCESS_ATLAS_COLUMNS)
    }

    fn embedded(json: &str, columns: u32) -> Self {
        let m = Self::parse(json).expect("character manifest must parse");
        m.validate(columns)
            .expect("character manifest must match its atlas");
        m
    }

    /// Legacy 48-texel art names the sole row as `pivot.y`; M2+ art names
    /// the ground contact line (first empty row below the sole).
    pub fn pivot_is_contact_line(&self) -> bool {
        self.frame_height != 48
    }
}

// --- Minimal JSON reader ------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
enum Json {
    Null,
    Bool(bool),
    Num(f64),
    Str(String),
    Arr(Vec<Json>),
    Obj(BTreeMap<String, Json>),
}

impl Json {
    fn as_object(&self) -> Option<&BTreeMap<String, Json>> {
        match self {
            Json::Obj(o) => Some(o),
            _ => None,
        }
    }
    fn as_array(&self) -> Option<&Vec<Json>> {
        match self {
            Json::Arr(a) => Some(a),
            _ => None,
        }
    }
    fn as_f64(&self) -> Option<f64> {
        match self {
            Json::Num(n) => Some(*n),
            _ => None,
        }
    }
    fn as_bool(&self) -> Option<bool> {
        match self {
            Json::Bool(b) => Some(*b),
            _ => None,
        }
    }
    fn as_str(&self) -> Option<&str> {
        match self {
            Json::Str(s) => Some(s),
            _ => None,
        }
    }

    fn parse(text: &str) -> Result<Json, String> {
        let mut p = Parser {
            s: text.as_bytes(),
            i: 0,
        };
        let v = p.value()?;
        p.ws();
        if p.i != p.s.len() {
            return Err(format!("trailing characters at byte {}", p.i));
        }
        Ok(v)
    }
}

struct Parser<'a> {
    s: &'a [u8],
    i: usize,
}

impl Parser<'_> {
    fn ws(&mut self) {
        while self.i < self.s.len() && self.s[self.i].is_ascii_whitespace() {
            self.i += 1;
        }
    }

    fn eat(&mut self, c: u8) -> Result<(), String> {
        self.ws();
        if self.s.get(self.i) == Some(&c) {
            self.i += 1;
            Ok(())
        } else {
            Err(format!("expected `{}` at byte {}", c as char, self.i))
        }
    }

    fn value(&mut self) -> Result<Json, String> {
        self.ws();
        match self.s.get(self.i) {
            Some(b'{') => self.object(),
            Some(b'[') => self.array(),
            Some(b'"') => self.string().map(Json::Str),
            Some(b't') => self.word("true", Json::Bool(true)),
            Some(b'f') => self.word("false", Json::Bool(false)),
            Some(b'n') => self.word("null", Json::Null),
            Some(c) if *c == b'-' || c.is_ascii_digit() => self.number(),
            _ => Err(format!("unexpected input at byte {}", self.i)),
        }
    }

    fn word(&mut self, w: &str, v: Json) -> Result<Json, String> {
        if self.s[self.i..].starts_with(w.as_bytes()) {
            self.i += w.len();
            Ok(v)
        } else {
            Err(format!("bad literal at byte {}", self.i))
        }
    }

    fn number(&mut self) -> Result<Json, String> {
        let start = self.i;
        while self.i < self.s.len()
            && (self.s[self.i].is_ascii_digit() || b"+-.eE".contains(&self.s[self.i]))
        {
            self.i += 1;
        }
        std::str::from_utf8(&self.s[start..self.i])
            .ok()
            .and_then(|t| t.parse::<f64>().ok())
            .map(Json::Num)
            .ok_or(format!("bad number at byte {start}"))
    }

    fn string(&mut self) -> Result<String, String> {
        self.eat(b'"')?;
        let mut out = String::new();
        loop {
            let c = *self.s.get(self.i).ok_or("unterminated string")?;
            self.i += 1;
            match c {
                b'"' => return Ok(out),
                b'\\' => {
                    let e = *self.s.get(self.i).ok_or("bad escape")?;
                    self.i += 1;
                    out.push(match e {
                        b'"' => '"',
                        b'\\' => '\\',
                        b'/' => '/',
                        b'n' => '\n',
                        b't' => '\t',
                        _ => return Err("unsupported escape".into()),
                    });
                }
                _ => {
                    // Copy the full UTF-8 sequence starting at this byte.
                    let start = self.i - 1;
                    let mut end = self.i;
                    while end < self.s.len() && (self.s[end] & 0xC0) == 0x80 {
                        end += 1;
                    }
                    out.push_str(
                        std::str::from_utf8(&self.s[start..end]).map_err(|e| e.to_string())?,
                    );
                    self.i = end;
                }
            }
        }
    }

    fn array(&mut self) -> Result<Json, String> {
        self.eat(b'[')?;
        let mut items = Vec::new();
        self.ws();
        if self.s.get(self.i) == Some(&b']') {
            self.i += 1;
            return Ok(Json::Arr(items));
        }
        loop {
            items.push(self.value()?);
            self.ws();
            match self.s.get(self.i) {
                Some(b',') => self.i += 1,
                Some(b']') => {
                    self.i += 1;
                    return Ok(Json::Arr(items));
                }
                _ => return Err(format!("expected `,` or `]` at byte {}", self.i)),
            }
        }
    }

    fn object(&mut self) -> Result<Json, String> {
        self.eat(b'{')?;
        let mut map = BTreeMap::new();
        self.ws();
        if self.s.get(self.i) == Some(&b'}') {
            self.i += 1;
            return Ok(Json::Obj(map));
        }
        loop {
            self.ws();
            let k = self.string()?;
            self.eat(b':')?;
            let v = self.value()?;
            map.insert(k, v);
            self.ws();
            match self.s.get(self.i) {
                Some(b',') => self.i += 1,
                Some(b'}') => {
                    self.i += 1;
                    return Ok(Json::Obj(map));
                }
                _ => return Err(format!("expected `,` or `}}` at byte {}", self.i)),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prince_manifest_parses_and_matches_atlas() {
        let m = AnimationManifest::prince();
        assert_eq!(m.character_id, "prince_debug");
        assert_eq!((m.frame_width, m.frame_height), (32, 48));
        assert_eq!(m.pivot, (16.0, 44.0));
        for name in ["idle", "run", "jump", "fall"] {
            assert!(m.clip(name).is_some(), "missing clip {name}");
        }
        assert_eq!(m.clip("run").unwrap().frames, vec![4, 5, 6, 7, 8, 9]);
        assert!(m.clip("idle").unwrap().looping);
        assert!(!m.clip("jump").unwrap().looping);
    }

    #[test]
    fn m3_hero_and_princess_manifests_match_their_atlases() {
        let cases: [(AnimationManifest, &[u8], u32); 2] = [
            (
                AnimationManifest::hero(),
                include_bytes!("../../../assets/characters/prince_m3.png"),
                HERO_ATLAS_COLUMNS,
            ),
            (
                AnimationManifest::princess(),
                include_bytes!("../../../assets/characters/princess_m3.png"),
                PRINCESS_ATLAS_COLUMNS,
            ),
        ];
        for (m, png, cols) in cases {
            assert_eq!((m.frame_width, m.frame_height), (96, 96));
            assert_eq!(m.pivot, (48.0, 88.0));
            assert!(m.pivot_is_contact_line());
            for name in ["idle", "run", "jump", "air", "fall", "landing"] {
                assert!(m.clip(name).is_some(), "{}: missing {name}", m.character_id);
            }
            let w = u32::from_be_bytes(png[16..20].try_into().unwrap());
            let h = u32::from_be_bytes(png[20..24].try_into().unwrap());
            assert_eq!((w, h), (m.frame_width * cols, m.frame_height));
        }
    }

    #[test]
    fn atlas_png_dimensions_match_manifest_grid() {
        // Read width/height from the PNG IHDR chunk (bytes 16..24).
        let png = include_bytes!("../../../assets/characters/prince_debug.png");
        assert_eq!(&png[1..4], b"PNG");
        let w = u32::from_be_bytes(png[16..20].try_into().unwrap());
        let h = u32::from_be_bytes(png[20..24].try_into().unwrap());
        let m = AnimationManifest::prince();
        assert_eq!(w, m.frame_width * PRINCE_ATLAS_COLUMNS);
        assert_eq!(h, m.frame_height);
    }

    #[test]
    fn validation_rejects_out_of_range_frames() {
        let bad = r#"{"schema_version":1,"character_id":"x","frame_width":32,
            "frame_height":48,"animations":{"idle":{"frames":[0,14],"fps":5,"loop":true}}}"#;
        let m = AnimationManifest::parse(bad).unwrap();
        assert!(m.validate(PRINCE_ATLAS_COLUMNS).is_err());
    }

    #[test]
    fn parser_rejects_malformed_json() {
        assert!(AnimationManifest::parse("{").is_err());
        assert!(AnimationManifest::parse(r#"{"schema_version":2}"#).is_err());
        assert!(Json::parse("[1, 2,]").is_err());
        assert_eq!(
            Json::parse(r#"{"a":[true,null,-1.5e1,"\"x\""]}"#).unwrap(),
            Json::Obj(BTreeMap::from([(
                "a".to_string(),
                Json::Arr(vec![
                    Json::Bool(true),
                    Json::Null,
                    Json::Num(-15.0),
                    Json::Str("\"x\"".into())
                ])
            )]))
        );
    }
}
