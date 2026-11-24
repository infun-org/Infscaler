use std::collections::HashMap;
use std::path::{Path, PathBuf};

use serde::Deserialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BackendKind {
    Ncnn,
    Onnx,
}

impl BackendKind {
    pub fn label(self) -> &'static str {
        match self {
            Self::Ncnn => "NCNN",
            Self::Onnx => "ONNX",
        }
    }
}

#[derive(Debug, Clone)]
pub enum ModelFiles {
    Ncnn { param: PathBuf, bin: PathBuf },
    Onnx { path: PathBuf },
}

#[derive(Debug, Clone)]
pub struct ModelInfo {
    pub name: String,
    pub scale: u32,
    pub license: String,
    pub source: String,
    pub notes: String,
    pub sha256: String,
    pub denoise: bool,
    pub broken_reason: Option<String>,
    pub files: ModelFiles,
}

impl ModelInfo {
    pub fn is_broken(&self) -> bool {
        self.broken_reason.is_some()
    }
}

impl ModelInfo {
    /// Friendly label for the GUI: "what it is for (short model name)".
    pub fn display_name(&self, lang: crate::i18n::Lang) -> String {
        crate::i18n::model_display_name(lang, &self.name).unwrap_or_else(|| self.name.clone())
    }

    pub fn kind(&self) -> BackendKind {
        match self.files {
            ModelFiles::Ncnn { .. } => BackendKind::Ncnn,
            ModelFiles::Onnx { .. } => BackendKind::Onnx,
        }
    }

    pub fn is_onnx(&self) -> bool {
        matches!(self.files, ModelFiles::Onnx { .. })
    }

    pub fn files_present(&self) -> bool {
        match &self.files {
            ModelFiles::Ncnn { param, bin } => param.is_file() && bin.is_file(),
            ModelFiles::Onnx { path } => path.is_file(),
        }
    }

    pub fn location(&self) -> String {
        match &self.files {
            ModelFiles::Ncnn { param, bin } => format!("{} / {}", param.display(), bin.display()),
            ModelFiles::Onnx { path } => path.display().to_string(),
        }
    }
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
    #[serde(default)]
    sha256: String,
    #[serde(default)]
    denoise: bool,
    #[serde(default)]
    broken_reason: Option<String>,
}

/// Scale encoded in names such as `ultrasharp-4x`, `4xNomos8kSC`, `model-x2`.
pub fn infer_scale(name: &str) -> u32 {
    let n = name.to_ascii_lowercase();
    if n.contains("scunet") {
        return 1;
    }
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
        ("x1", 1),
        ("1x", 1),
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

pub fn denoise_model(models: &[ModelInfo]) -> Option<&ModelInfo> {
    models
        .iter()
        .find(|m| m.denoise && !m.is_broken() && m.files_present())
}

pub fn load_models(root: &Path) -> Vec<ModelInfo> {
    let dir = root.join("models");
    let manifest = read_manifest(&dir.join("manifest.json"));
    let mut found: Vec<(String, ModelFiles)> = Vec::new();
    let Ok(entries) = std::fs::read_dir(&dir) else {
        return Vec::new();
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let Some(stem) = path
            .file_stem()
            .and_then(|s| s.to_str())
            .map(str::to_string)
        else {
            continue;
        };
        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| e.to_ascii_lowercase());
        match ext.as_deref() {
            Some("param") => {
                let bin = dir.join(format!("{stem}.bin"));
                if bin.is_file() {
                    found.push((stem, ModelFiles::Ncnn { param: path, bin }));
                }
            }
            Some("onnx") => {
                if stem == "face_detection_yunet" || stem == "RestoreFormerPlusPlus" {
                    continue;
                }
                let ncnn = dir.join(format!("{stem}.param"));
                // A same-named NCNN pair keeps the Vulkan path. The ONNX file is ignored.
                if !ncnn.is_file() {
                    found.push((stem, ModelFiles::Onnx { path }));
                }
            }
            _ => {}
        }
    }
    found.sort_by(|a, b| a.0.cmp(&b.0));
    found
        .into_iter()
        .map(|(name, files)| inflate(&name, files, manifest.get(&name)))
        .collect()
}

fn inflate(name: &str, files: ModelFiles, meta: Option<&ManifestEntry>) -> ModelInfo {
    let denoise_name = name.to_ascii_lowercase().contains("scunet");
    if let Some(meta) = meta {
        ModelInfo {
            scale: meta.scale.max(1),
            license: fallback(&meta.license, "see models/MODELS.md"),
            source: meta.source.clone(),
            notes: meta.notes.clone(),
            sha256: meta.sha256.clone(),
            denoise: meta.denoise || denoise_name,
            broken_reason: meta.broken_reason.clone(),
            name: name.to_string(),
            files,
        }
    } else {
        ModelInfo {
            scale: infer_scale(name),
            license: "see models/MODELS.md".into(),
            source: String::new(),
            notes: "Custom model (no manifest entry).".into(),
            sha256: String::new(),
            denoise: denoise_name,
            broken_reason: None,
            name: name.to_string(),
            files,
        }
    }
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
        assert_eq!(infer_scale("spanx4_ch48"), 4);
        assert_eq!(infer_scale("scunet_color_real"), 1);
        assert_eq!(infer_scale("photo"), 4);
    }
}
