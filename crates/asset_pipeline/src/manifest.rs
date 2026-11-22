use std::collections::HashSet;

use serde::Deserialize;
use serde_json::{Map, Value};

#[derive(Clone, Copy)]
pub enum Kind {
    Animation,
    Level,
}

type Object = Map<String, Value>;

fn object<'a>(
    value: &'a Value,
    path: &str,
    required: &[&str],
    optional: &[&str],
) -> Result<&'a Object, String> {
    let obj = value
        .as_object()
        .ok_or_else(|| format!("{path} must be an object"))?;
    for key in required {
        if !obj.contains_key(*key) {
            return Err(format!("{path} is missing field {key:?}"));
        }
    }
    for key in obj.keys() {
        if !required.contains(&key.as_str()) && !optional.contains(&key.as_str()) {
            return Err(format!("{path} contains unknown field {key:?}"));
        }
    }
    Ok(obj)
}

fn integer(value: &Value, path: &str, min: u64) -> Result<u64, String> {
    // JSON Schema also treats whole-valued decimal numbers as integers.
    let parsed = value.as_u64().or_else(|| {
        let n = value.as_f64()?;
        (n.fract() == 0.0 && n >= 0.0 && n < u64::MAX as f64).then_some(n as u64)
    });
    parsed
        .filter(|&n| n >= min)
        .ok_or_else(|| format!("{path} must be an integer >= {min} representable as u64"))
}

fn header(obj: &Object, id_field: &str) -> Result<(), String> {
    if integer(&obj["schema_version"], "schema_version", 1)? != 1 {
        return Err("schema_version must be 1".into());
    }
    let id = obj[id_field].as_str().unwrap_or_default();
    if !id.starts_with(|c: char| c.is_ascii_lowercase())
        || !id
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_')
    {
        return Err(format!("{id_field} must match ^[a-z][a-z0-9_]*$"));
    }
    Ok(())
}

fn character(value: &Value) -> Result<(), String> {
    let obj = object(
        value,
        "manifest",
        &[
            "schema_version",
            "character_id",
            "frame_width",
            "frame_height",
            "animations",
        ],
        &["pivot"],
    )?;
    header(obj, "character_id")?;
    for dim in ["frame_width", "frame_height"] {
        integer(&obj[dim], dim, 1)?;
    }
    if let Some(pivot) = obj.get("pivot") {
        let pivot = object(pivot, "pivot", &["x", "y"], &[])?;
        for axis in ["x", "y"] {
            if !pivot[axis].is_number() {
                return Err(format!("pivot.{axis} must be a number"));
            }
        }
    }
    let animations = obj["animations"]
        .as_object()
        .filter(|a| !a.is_empty())
        .ok_or("animations must be a nonempty object")?;
    for (name, clip) in animations {
        let path = format!("animations[{name:?}]");
        let clip = object(clip, &path, &["fps", "frames", "loop"], &[])?;
        if !clip["fps"]
            .as_f64()
            .is_some_and(|fps| fps > 0.0 && fps <= 60.0)
        {
            return Err(format!("{path}.fps must be greater than 0 and at most 60"));
        }
        if !clip["loop"].is_boolean() {
            return Err(format!("{path}.loop must be a boolean"));
        }
        let frames = clip["frames"]
            .as_array()
            .filter(|f| !f.is_empty())
            .ok_or_else(|| format!("{path}.frames must be a nonempty array"))?;
        let mut seen = HashSet::new();
        for (i, frame) in frames.iter().enumerate() {
            let index = integer(frame, &format!("{path}.frames[{i}]"), 0)?;
            if !seen.insert(index) {
                return Err(format!("{path}.frames[{i}] repeats index {index}"));
            }
        }
    }
    Ok(())
}

fn level(value: &Value) -> Result<(), String> {
    let obj = object(
        value,
        "level",
        &[
            "schema_version",
            "id",
            "tile_size",
            "width",
            "height",
            "spawn",
            "solids",
        ],
        &[],
    )?;
    header(obj, "id")?;
    if !matches!(integer(&obj["tile_size"], "tile_size", 1)?, 16 | 32) {
        return Err("tile_size must be 16 or 32".into());
    }
    let width = integer(&obj["width"], "width", 8)?;
    let height = integer(&obj["height"], "height", 4)?;
    let spawn = object(&obj["spawn"], "spawn", &["x", "y"], &[])?;
    let sx = integer(&spawn["x"], "spawn.x", 0)?;
    let sy = integer(&spawn["y"], "spawn.y", 0)?;
    if sx >= width || sy >= height {
        return Err("spawn is outside the level".into());
    }
    let solids = obj["solids"].as_array().ok_or("solids must be an array")?;
    for (i, solid) in solids.iter().enumerate() {
        let path = format!("solids[{i}]");
        let rect = object(solid, &path, &["x", "y", "w", "h"], &[])?;
        let coord = |key: &str, min| integer(&rect[key], &format!("{path}.{key}"), min);
        let (x, y, w, h) = (
            coord("x", 0)?,
            coord("y", 0)?,
            coord("w", 1)?,
            coord("h", 1)?,
        );
        // Check the remaining extent before adding, even for extreme input.
        if x >= width || w > width - x || y >= height || h > height - y {
            return Err(format!("{path} extends outside the level"));
        }
        if (x..x + w).contains(&sx) && (y..y + h).contains(&sy) {
            return Err(format!("spawn intersects {path}"));
        }
    }
    Ok(())
}

pub fn validate(kind: Kind, data: &[u8]) -> Result<(), String> {
    let mut input = serde_json::Deserializer::from_slice(data);
    let value = Value::deserialize(&mut input).map_err(|e| e.to_string())?;
    input.end().map_err(|e| e.to_string())?;
    match kind {
        Kind::Animation => character(&value),
        Kind::Level => level(&value),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn animation() -> Value {
        json!({"schema_version":1,"character_id":"prince_debug","frame_width":32,"frame_height":48,
            "animations":{"idle":{"frames":[0,1],"fps":6,"loop":true}}})
    }

    fn course() -> Value {
        json!({"schema_version":1,"id":"landing","tile_size":16,"width":8,"height":4,
            "spawn":{"x":2,"y":1},"solids":[{"x":0,"y":1,"w":2,"h":3}]})
    }

    #[test]
    fn valid_clips_can_share_frames_and_use_fractional_fps() {
        let mut a = animation();
        a["pivot"] = json!({"x":-1.25,"y":12});
        a["animations"]["jump"] = json!({"frames":[1.0],"fps":0.5,"loop":false});
        assert_eq!(character(&a), Ok(()));
    }

    #[test]
    fn missing_empty_and_repeated_frames() {
        let mut a = animation();
        a["animations"]["idle"]
            .as_object_mut()
            .unwrap()
            .remove("frames");
        assert!(character(&a).unwrap_err().contains("frames"));
        for frames in [
            json!([]),
            json!([2, 1, 2]),
            json!([-1]),
            json!([0.5]),
            json!(0),
        ] {
            a["animations"]["idle"]["frames"] = frames;
            assert!(character(&a).unwrap_err().contains("frames"));
        }
    }

    #[test]
    fn fps_limits() {
        let mut a = animation();
        for fps in [json!(0), json!(-1), json!(60.01), json!("12")] {
            a["animations"]["idle"]["fps"] = fps;
            assert!(character(&a).unwrap_err().contains(".fps"));
        }
        a["animations"]["idle"]["fps"] = json!(60);
        assert_eq!(character(&a), Ok(()));
    }

    #[test]
    fn invalid_animation_fields() {
        for (path, bad) in [
            ("/schema_version", json!(2)),
            ("/character_id", json!("1Prince")),
            ("/frame_width", json!(0)),
            ("/frame_height", json!(-2)),
            ("/animations", json!({})),
            ("/animations/idle/loop", json!(1)),
        ] {
            let mut a = animation();
            *a.pointer_mut(path).unwrap() = bad;
            assert!(character(&a).is_err(), "{path}");
        }
        for pivot in [
            json!(null),
            json!([1, 2]),
            json!({"x":0}),
            json!({"x":0,"y":0,"z":1}),
        ] {
            let mut a = animation();
            a["pivot"] = pivot;
            assert!(character(&a).unwrap_err().contains("pivot"));
        }
    }

    #[test]
    fn spawn_and_solid_edges() {
        let mut l = course();
        assert_eq!(level(&l), Ok(())); // spawn touches the exclusive right edge
        l["spawn"] = json!({"x":0,"y":1});
        assert!(
            level(&l)
                .unwrap_err()
                .contains("spawn intersects solids[0]")
        );
        l["spawn"] = json!({"x":8,"y":0});
        assert!(level(&l).unwrap_err().contains("spawn is outside"));
        l["spawn"] = json!({"x":0,"y":0});
        l["solids"] = json!([]);
        assert_eq!(level(&l), Ok(()));
    }

    #[test]
    fn invalid_level_fields_and_extents() {
        for (path, bad) in [
            ("/tile_size", json!(24)),
            ("/width", json!(7)),
            ("/height", json!(3)),
            ("/spawn/x", json!(-1)),
            ("/solids/0/w", json!(0)),
            ("/solids/0/h", json!(-1)),
            ("/solids/0/x", json!(u64::MAX)),
            ("/solids/0/h", json!(u64::MAX)),
            ("/solids/0/w", json!(9)),
            ("/solids/0/h", json!(4)),
        ] {
            let mut l = course();
            *l.pointer_mut(path).unwrap() = bad;
            assert!(level(&l).is_err(), "{path}");
        }
    }
}
