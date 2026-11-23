use std::collections::HashMap;
use std::path::{Path, PathBuf};

use serde::Deserialize;

#[derive(Debug, Clone)]
pub struct ModelInfo {
    pub name: String,
    pub scale: u32,
    pub license: String,
    pub source: String,
    pub notes: String,
    pub param: PathBuf,
    pub bin: PathBuf,
}

#[derive(Debug, Deserialize)]
struct ManifestEntry {
    scale: u32,
    #[serde(default)]
    license: String,
    #[serde(default)]
    source: String,
    #[serde(default)]
    notes: String,
}

/// Scale encoded in names such as `ultrasharp-4x`, `4xNomos8kSC`, `model-x2`.
pub fn infer_scale(name: &str) -> u32 {
    let n = name.to_ascii_lowercase();
    // Longer tokens first so `x16` does not match as `x1`.
    const PATTERNS: &[(&str, u32)] = &[
        ("x16", 16),
        ("16x", 16),
        ("x8", 8),
        ("8x", 8),
        ("x4", 4),
        ("4x", 4),
        ("x3", 3),
        ("3x", 3),
        ("x2", 2),
        ("2x", 2),
    ];
    for (pat, scale) in PATTERNS {
        if n.contains(pat) {
            return *scale;
        }
    }
    4
}

/// Official realesrgan-ncnn-vulkan only accepts these name fragments.
pub fn cpu_backend_supports(name: &str) -> bool {
    const KNOWN: &[&str] = &[
        "realesrgan-x4plus-anime",
        "realesrgan-x4plus",
        "realesrnet-x4plus",
        "realesr-animevideov3",
    ];
    KNOWN.iter().any(|k| name.contains(k))
}

pub fn load_models(root: &Path) -> Vec<ModelInfo> {
    let dir = root.join("models");
    let manifest = read_manifest(&dir.join("manifest.json"));
    let mut names = Vec::new();
    let Ok(entries) = std::fs::read_dir(&dir) else {
        return Vec::new();
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("param") {
            continue;
        }
        let Some(stem) = path.file_stem().and_then(|s| s.to_str()) else {
            continue;
        };
        let bin = dir.join(format!("{stem}.bin"));
        if bin.is_file() {
            names.push(stem.to_string());
        }
    }
    names.sort();
    names
        .into_iter()
        .map(|name| {
            let param = dir.join(format!("{name}.param"));
            let bin = dir.join(format!("{name}.bin"));
            if let Some(meta) = manifest.get(&name) {
                ModelInfo {
                    name,
                    scale: meta.scale.max(1),
                    license: fallback(&meta.license, "see models/MODELS.md"),
                    source: meta.source.clone(),
                    notes: meta.notes.clone(),
                    param,
                    bin,
                }
            } else {
                ModelInfo {
                    scale: infer_scale(&name),
                    license: "see models/MODELS.md".into(),
                    source: String::new(),
                    notes: "Custom model (no manifest entry).".into(),
                    name,
                    param,
                    bin,
                }
            }
        })
        .collect()
}

fn fallback(value: &str, default: &str) -> String {
    if value.is_empty() {
        default.to_string()
    } else {
        value.to_string()
    }
}

fn read_manifest(path: &Path) -> HashMap<String, ManifestEntry> {
    let Ok(text) = std::fs::read_to_string(path) else {
        return HashMap::new();
    };
    serde_json::from_str(&text).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::infer_scale;

    #[test]
    fn scale_from_common_names() {
        assert_eq!(infer_scale("realesrgan-x4plus"), 4);
        assert_eq!(infer_scale("ultrasharp-4x"), 4);
        assert_eq!(infer_scale("4xNomos8kSC"), 4);
        assert_eq!(infer_scale("realesr-animevideov3-x2"), 2);
        assert_eq!(infer_scale("realesr-animevideov3-x3"), 3);
        assert_eq!(infer_scale("photo"), 4);
    }
}
