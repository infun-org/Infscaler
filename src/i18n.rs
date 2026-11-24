//! Small English / Vietnamese string table for the GUI.
//! CLI output stays in English.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Lang {
    En,
    #[default]
    Vi,
}

impl Lang {
    pub fn label(self) -> &'static str {
        match self {
            Self::En => "English",
            Self::Vi => "Tiếng Việt",
        }
    }

    pub fn all() -> [Self; 2] {
        [Self::Vi, Self::En]
    }
}

/// `(key, english, vietnamese)` — every user-visible GUI string lives here.
pub const ENTRIES: &[(&str, &str, &str)] = &[
    (
        "drop_or_open",
        "Drop a photo, or open one.",
        "Thả một ảnh vào đây, hoặc mở ảnh.",
    ),
    (
        "missing_project_folder_long",
        "Could not find the InfScaler folder (scripts/setup.sh). Start the app from the project directory, or set INFSCALER_HOME.",
        "Không tìm thấy thư mục InfScaler (scripts/setup.sh). Hãy mở ứng dụng từ thư mục dự án, hoặc đặt INFSCALER_HOME.",
    ),
    (
        "no_models_yet",
        "No models yet. Run scripts/setup.sh, then Reload.",
        "Chưa có model. Chạy scripts/setup.sh, rồi bấm Tải lại.",
    ),
    ("upscaling_pct", "Upscaling {}%", "Đang xử lý {}%"),
    ("saved", "Saved {} ({}×{})", "Đã lưu {} ({}×{})"),
    (
        "saved_preview_failed",
        "Saved the file, but the preview failed: {}",
        "Đã lưu file, nhưng không xem trước được: {}",
    ),
    ("failed", "Failed", "Thất bại"),
    ("batch_complete", "Batch complete", "Đã xử lý xong thư mục"),
    (
        "batch_stopped",
        "Batch stopped: {}",
        "Đã dừng xử lý thư mục: {}",
    ),
    (
        "thread_exited",
        "The processing thread exited unexpectedly.",
        "Luồng xử lý đã dừng đột ngột.",
    ),
    (
        "could_not_read_size",
        "Could not read image size: {}",
        "Không đọc được kích thước ảnh: {}",
    ),
    ("loaded_size", "Loaded {}×{}", "Đã mở {}×{}"),
    ("loaded", "Loaded", "Đã mở"),
    ("open_photo", "Open photo", "Mở ảnh"),
    ("images_filter", "Images", "Ảnh"),
    ("save_enhanced", "Save enhanced image", "Lưu ảnh đã làm nét"),
    (
        "project_folder_missing",
        "Project folder not found.",
        "Không tìm thấy thư mục dự án.",
    ),
    (
        "no_model_selected",
        "No model selected. Run scripts/setup.sh.",
        "Chưa chọn model. Hãy chạy scripts/setup.sh.",
    ),
    (
        "model_files_missing",
        "Model files for {} are missing ({}).",
        "Thiếu file model {} ({}).",
    ),
    (
        "choose_input",
        "Choose an input image first.",
        "Hãy chọn ảnh nguồn trước.",
    ),
    (
        "choose_output",
        "Choose an output path.",
        "Hãy chọn đường dẫn lưu ảnh.",
    ),
    (
        "tile_size_invalid",
        "Tile size must be 0 (auto) or at least 32.",
        "Kích thước ô phải là 0 (tự động) hoặc ít nhất 32.",
    ),
    ("starting", "Starting…", "Đang bắt đầu…"),
    ("cancelling", "Cancelling…", "Đang hủy…"),
    (
        "tagline",
        "local photo enhancement",
        "làm nét ảnh ngay trên máy",
    ),
    ("reload_models", "Reload models", "Tải lại model"),
    ("model_count", "{} model(s)", "{} model"),
    ("photo", "Photo", "Ảnh"),
    ("open", "Open…", "Mở…"),
    (
        "or_drop_file",
        "or drop a file on the window",
        "hoặc thả file vào cửa sổ",
    ),
    ("output_folder", "Output: {}", "Thư mục lưu: {}"),
    (
        "output_folder_auto",
        "Output folder: automatic",
        "Thư mục lưu: tự động",
    ),
    (
        "choose_output_folder",
        "Choose output folder",
        "Chọn thư mục lưu",
    ),
    ("onnx_threads", "ONNX threads", "Luồng ONNX"),
    ("open_folder", "Open folder…", "Mở thư mục…"),
    ("open_image_folder", "Open image folder", "Mở thư mục ảnh"),
    (
        "queued_top_level",
        "Queued {} top-level images",
        "Đã thêm {} ảnh (chỉ thư mục hiện tại)",
    ),
    ("queue_count", "Queue: {} image(s)", "Hàng đợi: {} ảnh"),
    (
        "preset_general",
        "Animals & general",
        "Động vật & chung",
    ),
    ("preset_people", "People", "Người"),
    ("preset_fast", "Fast", "Nhanh"),
    ("jobs_one", "1 job", "1 ảnh một lúc"),
    ("jobs_n", "{} jobs", "{} ảnh song song"),
    ("and_more", "… and {} more", "… và {} ảnh nữa"),
    (
        "process_queued",
        "Process queued folder",
        "Xử lý hàng đợi",
    ),
    ("model", "Model", "Model"),
    (
        "auto_choose",
        "Auto (choose per image)",
        "Tự động (chọn theo ảnh)",
    ),
    ("none", "none", "không có"),
    ("broken_suffix", " · BROKEN", " · hỏng"),
    (
        "no_models_in_folder",
        "No .param + .bin or .onnx models in models/",
        "Không có model .param + .bin hoặc .onnx trong thư mục models/",
    ),
    ("broken_reason", "BROKEN: {}", "Hỏng: {}"),
    (
        "auto_hint",
        "People → RealWebPhoto DAT · no faces → Nomos2 OTF",
        "Người → DAT · không có mặt → Nomos2 OTF",
    ),
    ("output_size", "Output size", "Kích thước đầu ra"),
    (
        "original_sharpen",
        "Original (sharpen)",
        "Gốc (làm nét)",
    ),
    (
        "original_sharpen_tip",
        "Recommended: enhance, then scale back to the original pixel size",
        "Nên dùng: làm nét rồi thu về đúng kích thước gốc",
    ),
    (
        "hint_keep_size",
        "Runs the {}× model, then Lanczos3 back to the original pixels.",
        "Chạy model {}×, rồi thu về kích thước gốc bằng Lanczos3.",
    ),
    (
        "hint_native",
        "Writes the {}× model output.",
        "Ghi trực tiếp kết quả model {}×.",
    ),
    (
        "hint_rescale",
        "Runs the {}× model, then Lanczos3 to {}× the original.",
        "Chạy model {}×, rồi đổi kích thước thành {}× ảnh gốc bằng Lanczos3.",
    ),
    ("tile", "Tile", "Ô cắt"),
    ("tile_auto", "0 = auto", "0 = tự động"),
    ("overlap", "Overlap", "Vùng chồng"),
    ("onnx_seams", "ONNX seams", "Đường nối ONNX"),
    (
        "tile_hint",
        "NCNN uses the Vulkan tile. ONNX blends a Hann ramp on the CPU. DAT, DRCT, and SCUNet use their fixed input size and at least 48 px of overlap.",
        "NCNN dùng ô cắt Vulkan. ONNX hòa đường nối kiểu Hann trên CPU. DAT, DRCT và SCUNet dùng kích thước đầu vào cố định và chồng ít nhất 48 px.",
    ),
    ("gpu", "GPU", "GPU"),
    ("auto", "Auto", "Tự động"),
    ("cpu", "CPU", "CPU"),
    (
        "cpu_hint",
        "CPU runs ONNX models. NCNN binaries reject -g -1.",
        "CPU chạy các model ONNX. Bộ xử lý NCNN không nhận -g -1.",
    ),
    (
        "tta",
        "TTA (8 flips, slower)",
        "TTA (8 hướng lật, chậm hơn)",
    ),
    (
        "tta_onnx_hint",
        "This model runs on the CPU through ONNX Runtime. TTA stays off.",
        "Model này chạy trên CPU qua ONNX Runtime. TTA được tắt.",
    ),
    (
        "denoise_first",
        "Denoise first (SCUNet)",
        "Khử nhiễu trước (SCUNet)",
    ),
    (
        "face_restore",
        "Restore detected faces after enhancement",
        "Phục hồi khuôn mặt sau khi làm nét",
    ),
    ("face_strength", "Face strength", "Mức phục hồi mặt"),
    (
        "fast_auto",
        "Fast Auto (Nomos8kSC on GPU)",
        "Tự động nhanh (Nomos8kSC trên GPU)",
    ),
    ("save_as", "Save as", "Lưu thành"),
    ("browse", "Browse", "Chọn…"),
    (
        "last_result",
        "Last result {} × {} px",
        "Kết quả gần nhất {} × {} px",
    ),
    ("resume", "Resume", "Tiếp tục"),
    ("pause", "Pause", "Tạm dừng"),
    ("cancel", "Cancel", "Hủy"),
    ("enhance", "Enhance", "Làm nét"),
    (
        "drop_here",
        "Drop a photo of a person or animal here",
        "Thả ảnh người hoặc động vật vào đây",
    ),
    ("before", "Before", "Trước"),
    ("after", "After", "Sau"),
    ("original", "Original", "Gốc"),
    ("language", "Language", "Ngôn ngữ"),
    ("gpu_n", "GPU {}", "GPU {}"),
    // Model friendly names keep the short name in parentheses.
    (
        "model_nomos2",
        "Animals & general, best detail (Nomos2 OTF)",
        "Động vật & chung, chi tiết nhất (Nomos2 OTF)",
    ),
    (
        "model_dat2",
        "People, natural look (slow) (DAT2)",
        "Người, tự nhiên (chậm) (DAT2)",
    ),
    (
        "model_span",
        "Quick preview (SPAN)",
        "Xem trước nhanh (SPAN)",
    ),
    (
        "model_nomos8k",
        "Fast, GPU (Nomos8kSC)",
        "Nhanh, GPU (Nomos8kSC)",
    ),
    (
        "model_realesrgan",
        "Classic, GPU (Real-ESRGAN x4+)",
        "Cổ điển, GPU (Real-ESRGAN x4+)",
    ),
    (
        "model_ultrasharp",
        "Extra sharp, GPU, non-commercial (UltraSharp)",
        "Siêu nét, GPU, phi thương mại (UltraSharp)",
    ),
    (
        "model_scunet",
        "Denoise (SCUNet)",
        "Khử nhiễu (SCUNet)",
    ),
    (
        "model_drct",
        "High quality, disabled (DRCT-L)",
        "Chất lượng cao, đã tắt (DRCT-L)",
    ),
];

/// Fill `{}` placeholders in a template, left to right.
pub fn fill(template: &str, args: &[&str]) -> String {
    let mut out = String::new();
    let mut rest = template;
    for arg in args {
        if let Some(idx) = rest.find("{}") {
            out.push_str(&rest[..idx]);
            out.push_str(arg);
            rest = &rest[idx + 2..];
        }
    }
    out.push_str(rest);
    out
}

pub fn tr(lang: Lang, key: &str, args: &[&str]) -> String {
    fill(t(lang, key), args)
}

pub fn t(lang: Lang, key: &str) -> &'static str {
    for (k, en, vi) in ENTRIES {
        if *k == key {
            return match lang {
                Lang::En => en,
                Lang::Vi => vi,
            };
        }
    }
    "[?]"
}

pub fn model_display_name(lang: Lang, name: &str) -> Option<String> {
    let key = match name {
        "4xNomos2_otf_esrgan" => "model_nomos2",
        "4xRealWebPhoto_v4_dat2" => "model_dat2",
        "spanx4_ch48" => "model_span",
        "4xNomos8kSC" => "model_nomos8k",
        "realesrgan-x4plus" => "model_realesrgan",
        "ultrasharp-4x" => "model_ultrasharp",
        "scunet_color_real_psnr" => "model_scunet",
        "4xNomos2_hq_drct-l" => "model_drct",
        _ => return None,
    };
    Some(t(lang, key).to_string())
}

pub fn preset_label(lang: Lang, preset: &str) -> &'static str {
    match preset {
        "general" => t(lang, "preset_general"),
        "people" => t(lang, "preset_people"),
        "fast" => t(lang, "preset_fast"),
        _ => t(lang, "none"),
    }
}

pub fn jobs_label(lang: Lang, jobs: usize) -> String {
    if jobs == 1 {
        t(lang, "jobs_one").to_string()
    } else {
        tr(lang, "jobs_n", &[&jobs.to_string()])
    }
}

/// Re-express a status / stage / short error string in `lang`.
///
/// Known English engine stage lines (and their Vietnamese counterparts) are
/// rewritten; anything unrecognized is returned unchanged.
pub fn localize_runtime(lang: Lang, text: &str) -> String {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return text.to_string();
    }

    for (_key, en, vi) in ENTRIES {
        if trimmed == *en || trimmed == *vi {
            return match lang {
                Lang::En => (*en).to_string(),
                Lang::Vi => (*vi).to_string(),
            };
        }
    }

    if let Some(value) = take_prefix(trimmed, &["Upscaling ", "Đang xử lý "]) {
        if value.ends_with('%') {
            return tr(lang, "upscaling_pct", &[value.trim_end_matches('%')]);
        }
    }
    if let Some(rest) = take_prefix(
        trimmed,
        &[
            "Saved the file, but the preview failed: ",
            "Đã lưu file, nhưng không xem trước được: ",
        ],
    ) {
        return tr(lang, "saved_preview_failed", &[rest]);
    }
    if let Some(rest) = take_prefix(trimmed, &["Saved ", "Đã lưu "]) {
        return match lang {
            Lang::En => format!("Saved {rest}"),
            Lang::Vi => format!("Đã lưu {rest}"),
        };
    }
    if let Some(rest) = take_prefix(trimmed, &["Batch stopped: ", "Đã dừng xử lý thư mục: "])
    {
        return tr(lang, "batch_stopped", &[rest]);
    }
    if let Some(rest) = take_prefix(
        trimmed,
        &[
            "Could not read image size: ",
            "Không đọc được kích thước ảnh: ",
        ],
    ) {
        return tr(lang, "could_not_read_size", &[rest]);
    }
    if let Some(rest) = take_prefix(trimmed, &["Loaded ", "Đã mở "]) {
        return match lang {
            Lang::En => format!("Loaded {rest}"),
            Lang::Vi => format!("Đã mở {rest}"),
        };
    }
    if let Some(n) = take_prefix(trimmed, &["Queued ", "Đã thêm "])
        .and_then(|rest| rest.split_whitespace().next()?.parse::<usize>().ok())
    {
        return tr(lang, "queued_top_level", &[&n.to_string()]);
    }
    if let Some(n) = trimmed
        .split_whitespace()
        .next()
        .and_then(|w| w.parse::<usize>().ok())
        .filter(|_| {
            trimmed.ends_with("model(s)")
                || trimmed.ends_with(" model")
                || trimmed.ends_with(" model(s)")
        })
    {
        if trimmed == format!("{n} model(s)") || trimmed == format!("{n} model") {
            return tr(lang, "model_count", &[&n.to_string()]);
        }
    }
    if let Some(rest) = take_prefix(trimmed, &["BROKEN: ", "Hỏng: "]) {
        return tr(lang, "broken_reason", &[rest]);
    }
    if let Some(rest) = take_prefix(trimmed, &["Model files for ", "Thiếu file model "]) {
        // "NAME are missing (PATH)." or "NAME (PATH)."
        let (name, path) = if let Some(head) = rest.strip_suffix('.') {
            if let Some((name, path)) = head.split_once(" are missing (") {
                (name, path.trim_end_matches(')'))
            } else if let Some((name, path)) = head.split_once(" (") {
                (name, path.trim_end_matches(')'))
            } else {
                return text.to_string();
            }
        } else {
            return text.to_string();
        };
        return tr(lang, "model_files_missing", &[name, path]);
    }

    // Engine stage lines (also shown in the GUI status bar).
    if let Some(rest) = take_prefix(trimmed, &["Auto chose ", "Tự động chọn "]) {
        return match lang {
            Lang::En => format!("Auto chose {rest}"),
            Lang::Vi => format!("Tự động chọn {rest}"),
        };
    }
    if let Some(rest) = take_prefix(trimmed, &["Denoising with ", "Đang khử nhiễu bằng "]) {
        return match lang {
            Lang::En => format!("Denoising with {}", localize_job_details(Lang::En, rest)),
            Lang::Vi => format!(
                "Đang khử nhiễu bằng {}",
                localize_job_details(Lang::Vi, rest)
            ),
        };
    }
    if let Some(rest) = take_prefix(trimmed, &["Enhancing with ", "Đang làm nét bằng "]) {
        return match lang {
            Lang::En => format!("Enhancing with {}", localize_job_details(Lang::En, rest)),
            Lang::Vi => format!("Đang làm nét bằng {}", localize_job_details(Lang::Vi, rest)),
        };
    }
    if let Some(rest) = take_prefix(trimmed, &["Upscaling with ", "Đang phóng to bằng "]) {
        return match lang {
            Lang::En => format!("Upscaling with {rest}"),
            Lang::Vi => format!("Đang phóng to bằng {rest}"),
        };
    }
    if let Some(rest) = take_prefix(trimmed, &["Lanczos3 resize ", "Đổi kích thước Lanczos3 "])
    {
        return match lang {
            Lang::En => format!("Lanczos3 resize {rest}"),
            Lang::Vi => format!("Đổi kích thước Lanczos3 {rest}"),
        };
    }
    if let Some((n, model)) = parse_restore_faces(trimmed) {
        return match lang {
            Lang::En => format!("Restoring {n} face(s) with {model}"),
            Lang::Vi => format!("Đang phục hồi {n} khuôn mặt bằng {model}"),
        };
    }
    if trimmed == "Saving output" || trimmed == "Đang lưu ảnh" {
        return match lang {
            Lang::En => "Saving output".into(),
            Lang::Vi => "Đang lưu ảnh".into(),
        };
    }
    if trimmed == "Detecting faces in enhanced image"
        || trimmed == "Đang tìm khuôn mặt trên ảnh đã làm nét"
    {
        return match lang {
            Lang::En => "Detecting faces in enhanced image".into(),
            Lang::Vi => "Đang tìm khuôn mặt trên ảnh đã làm nét".into(),
        };
    }
    if trimmed == "NCNN output was black; retrying with 4xNomos2_otf_esrgan on ONNX Runtime CPU"
        || trimmed == "Đầu ra NCNN bị đen; thử lại với 4xNomos2_otf_esrgan trên ONNX Runtime CPU"
    {
        return match lang {
            Lang::En => {
                "NCNN output was black; retrying with 4xNomos2_otf_esrgan on ONNX Runtime CPU"
                    .into()
            }
            Lang::Vi => {
                "Đầu ra NCNN bị đen; thử lại với 4xNomos2_otf_esrgan trên ONNX Runtime CPU".into()
            }
        };
    }

    text.to_string()
}

fn take_prefix<'a>(text: &'a str, prefixes: &[&str]) -> Option<&'a str> {
    for prefix in prefixes {
        if let Some(rest) = text.strip_prefix(prefix) {
            return Some(rest);
        }
    }
    None
}

fn parse_restore_faces(text: &str) -> Option<(&str, &str)> {
    if let Some(rest) = text.strip_prefix("Restoring ") {
        if let Some((n, model)) = rest.split_once(" face(s) with ") {
            return Some((n, model));
        }
        if let Some((n, model)) = rest.split_once(" faces with ") {
            return Some((n, model));
        }
    }
    if let Some(rest) = text.strip_prefix("Đang phục hồi ") {
        if let Some((n, model)) = rest.split_once(" khuôn mặt bằng ") {
            return Some((n, model));
        }
    }
    None
}

fn localize_job_details(lang: Lang, details: &str) -> String {
    match lang {
        Lang::En => details
            .replace(" luồng", " threads")
            .replace(" ô ", " tile ")
            .replace(" chồng ", " overlap "),
        Lang::Vi => details
            .replace(" threads", " luồng")
            .replace(" tile ", " ô ")
            .replace(" overlap ", " chồng "),
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SettingsFile {
    lang: Lang,
}

pub fn settings_path() -> Option<PathBuf> {
    if let Some(xdg) = std::env::var_os("XDG_CONFIG_HOME") {
        let path = PathBuf::from(xdg);
        if !path.as_os_str().is_empty() {
            return Some(path.join("infscaler/settings.json"));
        }
    }
    std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".config/infscaler/settings.json"))
}

pub fn load_lang() -> Lang {
    let Some(path) = settings_path() else {
        return Lang::Vi;
    };
    let Ok(text) = std::fs::read_to_string(path) else {
        return Lang::Vi;
    };
    serde_json::from_str::<SettingsFile>(&text)
        .map(|s| s.lang)
        .unwrap_or(Lang::Vi)
}

pub fn save_lang(lang: Lang) {
    let Some(path) = settings_path() else {
        return;
    };
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if let Ok(text) = serde_json::to_string_pretty(&SettingsFile { lang }) {
        let _ = std::fs::write(path, text);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn every_key_has_nonempty_translation_in_both_languages() {
        let mut seen = HashSet::new();
        for (key, en, vi) in ENTRIES {
            assert!(!key.is_empty(), "empty key");
            assert!(seen.insert(*key), "duplicate i18n key: {key}");
            assert!(!en.is_empty(), "empty English string for {key}");
            assert!(!vi.is_empty(), "empty Vietnamese string for {key}");
        }
        assert!(!seen.is_empty());
        assert_eq!(t(Lang::En, "enhance"), "Enhance");
        assert_eq!(t(Lang::Vi, "enhance"), "Làm nét");
        assert_eq!(t(Lang::Vi, "missing-key-xyz"), "[?]");
    }

    #[test]
    fn known_models_have_friendly_names_in_both_languages() {
        let names = [
            "4xNomos2_otf_esrgan",
            "4xRealWebPhoto_v4_dat2",
            "spanx4_ch48",
            "4xNomos8kSC",
            "realesrgan-x4plus",
            "ultrasharp-4x",
            "scunet_color_real_psnr",
            "4xNomos2_hq_drct-l",
        ];
        for name in names {
            let en = model_display_name(Lang::En, name).unwrap();
            let vi = model_display_name(Lang::Vi, name).unwrap();
            assert!(en.contains('(') && en.ends_with(')'), "{en}");
            assert!(vi.contains('(') && vi.ends_with(')'), "{vi}");
            assert_ne!(en, vi, "{name} should differ across languages");
        }
        assert!(model_display_name(Lang::Vi, "unknown-model-xyz").is_none());
    }

    #[test]
    fn localize_runtime_roundtrips_common_status_lines() {
        let samples = [
            "Drop a photo, or open one.",
            "Starting…",
            "Failed",
            "Batch complete",
            "Upscaling 42.5%",
            "Saved /tmp/out.png (800×600)",
            "Loaded 1920×1080",
            "Queued 12 top-level images",
            "3 model(s)",
            "Auto chose 4xNomos2_otf_esrgan (general)",
            "Saving output",
            "Restoring 2 face(s) with RestoreFormerPlusPlus",
            "Enhancing with 4xNomos2_otf_esrgan (ONNX Runtime CPU, 4 threads, tile 128, overlap 16)",
        ];
        for sample in samples {
            let vi = localize_runtime(Lang::Vi, sample);
            assert_ne!(vi, sample, "expected a Vietnamese rewrite of {sample:?}");
            let back = localize_runtime(Lang::En, &vi);
            assert_eq!(back, sample, "roundtrip failed for {sample:?} via {vi:?}");
        }
        assert_eq!(
            localize_runtime(Lang::Vi, "something-unknown"),
            "something-unknown"
        );
    }

    #[test]
    fn default_lang_is_vietnamese() {
        assert_eq!(Lang::default(), Lang::Vi);
    }
}
