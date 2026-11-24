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
        "Could not find InfScaler's files. Open the app from the folder where it was installed.",
        "Không tìm thấy dữ liệu của InfScaler. Hãy mở ứng dụng từ thư mục đã cài.",
    ),
    (
        "no_models_yet",
        "No enhancement styles yet. Run setup, then click Reload list.",
        "Chưa có kiểu làm nét. Hãy chạy bước cài đặt, rồi bấm Tải lại danh sách.",
    ),
    (
        "upscaling_pct",
        "Enhancing {}%",
        "Đang xử lý {}%",
    ),
    ("saved", "Saved {} ({}×{})", "Đã lưu {} ({}×{})"),
    (
        "saved_preview_failed",
        "Saved the file, but the preview could not be shown: {}",
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
        "Processing stopped unexpectedly.",
        "Quá trình xử lý đã dừng đột ngột.",
    ),
    (
        "could_not_read_size",
        "Could not read the photo size: {}",
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
        "No style selected. Run setup first.",
        "Chưa chọn kiểu làm nét. Hãy chạy bước cài đặt trước.",
    ),
    (
        "model_files_missing",
        "Files for the {} style are missing ({}).",
        "Thiếu file của kiểu {} ({}).",
    ),
    (
        "choose_input",
        "Please choose a photo first.",
        "Hãy chọn một ảnh trước.",
    ),
    (
        "choose_output",
        "Please choose where to save the photo.",
        "Hãy chọn nơi lưu ảnh.",
    ),
    (
        "tile_size_invalid",
        "Piece size must be 0 (automatic) or at least 32.",
        "Kích thước mảnh phải là 0 (tự động) hoặc từ 32 trở lên.",
    ),
    ("starting", "Starting…", "Đang bắt đầu…"),
    ("cancelling", "Cancelling…", "Đang hủy…"),
    (
        "tagline",
        "local photo enhancement",
        "làm nét ảnh ngay trên máy",
    ),
    (
        "reload_models",
        "Reload list",
        "Tải lại danh sách",
    ),
    (
        "model_count",
        "{} style(s)",
        "{} kiểu",
    ),
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
    (
        "onnx_threads",
        "Processing threads",
        "Số luồng xử lý",
    ),
    ("open_folder", "Open folder…", "Mở thư mục…"),
    ("open_image_folder", "Open image folder", "Mở thư mục ảnh"),
    (
        "queued_top_level",
        "Queued {} images",
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
    (
        "jobs_one",
        "1 photo at a time",
        "1 ảnh một lúc",
    ),
    (
        "jobs_n",
        "{} photos at once",
        "{} ảnh cùng lúc",
    ),
    ("and_more", "… and {} more", "… và {} ảnh nữa"),
    (
        "process_queued",
        "Process queued folder",
        "Xử lý hàng đợi",
    ),
    (
        "model",
        "Style",
        "Kiểu làm nét",
    ),
    (
        "auto_choose",
        "Auto (choose per image)",
        "Tự động (chọn theo ảnh)",
    ),
    ("none", "none", "không có"),
    (
        "broken_suffix",
        " · disabled",
        " · tạm tắt",
    ),
    (
        "no_models_in_folder",
        "No enhancement styles are installed yet. Run setup, then click Reload list.",
        "Chưa có kiểu làm nét nào. Hãy chạy bước cài đặt, rồi bấm Tải lại danh sách.",
    ),
    (
        "broken_reason",
        "Disabled: {}",
        "Đã tắt: {}",
    ),

    ("output_size", "Output size", "Kích thước đầu ra"),
    (
        "original_sharpen",
        "Original (sharpen)",
        "Gốc (làm nét)",
    ),
    (
        "original_sharpen_tip",
        "Recommended: sharpen while keeping the original size",
        "Nên dùng: làm nét nhưng giữ nguyên kích thước",
    ),
    (
        "hint_keep_size",
        "Sharpens the photo, then brings it back to the original size.",
        "Làm nét ảnh rồi đưa về đúng kích thước ban đầu.",
    ),
    (
        "hint_native",
        "Saves the photo enlarged {}×.",
        "Lưu ảnh phóng to {} lần.",
    ),
    (
        "hint_rescale",
        "Sharpens the photo, then resizes it to {}× the original.",
        "Làm nét rồi đổi thành {} lần kích thước ảnh gốc.",
    ),
    (
        "tile",
        "Piece size",
        "Chia nhỏ ảnh",
    ),
    (
        "tile_auto",
        "0 = automatic",
        "0 = tự động",
    ),
    (
        "overlap",
        "Edge overlap",
        "Độ chồng mép",
    ),
    (
        "onnx_seams",
        "smooths the joins",
        "làm mượt chỗ nối",
    ),
    (
        "tile_hint",
        "Large photos are processed in pieces so the computer is not overloaded. If you see lines where pieces meet, increase the edge overlap.",
        "Ảnh lớn được xử lý từng mảnh để máy không bị quá tải. Nếu thấy vệt ở chỗ nối, hãy tăng độ chồng mép.",
    ),
    (
        "gpu",
        "Graphics card",
        "Card đồ họa",
    ),
    ("auto", "Auto", "Tự động"),
    ("cpu", "CPU", "CPU"),
    (
        "cpu_hint",
        "Only the fast styles use the graphics card; the others run on the processor.",
        "Chỉ các kiểu nhanh dùng card đồ họa; các kiểu còn lại chạy bằng bộ xử lý của máy.",
    ),
    (
        "tta",
        "Extra-careful mode (much slower)",
        "Xử lý kỹ hơn (chậm hơn nhiều)",
    ),
    (
        "tta_onnx_hint",
        "Extra-careful mode is only available for the graphics card styles.",
        "Chế độ xử lý kỹ chỉ có ở các kiểu dùng card đồ họa.",
    ),
    (
        "denoise_first",
        "Remove grain first",
        "Khử nhiễu hạt trước",
    ),
    (
        "face_restore",
        "Restore detected faces after enhancement",
        "Phục hồi khuôn mặt sau khi làm nét",
    ),
    ("face_strength", "Face strength", "Mức phục hồi mặt"),
    (
        "fast_auto",
        "Fast Auto (uses the graphics card)",
        "Tự động nhanh (dùng card đồ họa)",
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
    (
        "gpu_n",
        "Graphics card {}",
        "Card đồ họa {}",
    ),
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
    (
        "desc_auto",
        "Picks a style for each photo by itself: photos with people get the natural-look style (slow, about 80–110 seconds per small photo), other photos get the detailed style (about 17–34 seconds).",
        "Tự chọn kiểu phù hợp cho từng ảnh: ảnh có người dùng kiểu tự nhiên (chậm, khoảng 80–110 giây mỗi ảnh nhỏ), ảnh khác dùng kiểu chi tiết (khoảng 17–34 giây).",
    ),
    (
        "desc_nomos2",
        "Best for animals, landscapes and everyday photos; brings out fur, hair and fine texture. Medium speed, about 17–34 seconds per small photo.",
        "Hợp với ảnh động vật, phong cảnh và ảnh đời thường; làm rõ lông, tóc và chi tiết nhỏ. Tốc độ vừa, khoảng 17–34 giây mỗi ảnh nhỏ.",
    ),
    (
        "desc_dat2",
        "Best for portraits and photos of people; skin and faces stay natural, without a plastic look. Slow, about 80–110 seconds per small photo.",
        "Hợp với ảnh chân dung và ảnh có người; da và khuôn mặt giữ được vẻ tự nhiên, không bị bệt. Chậm, khoảng 80–110 giây mỗi ảnh nhỏ.",
    ),
    (
        "desc_span",
        "For a quick look before choosing a better style; the result is less sharp. Very fast, about 1 second per small photo.",
        "Dùng để xem thử nhanh trước khi chọn kiểu tốt hơn; ảnh ra kém nét hơn. Rất nhanh, khoảng 1 giây mỗi ảnh nhỏ.",
    ),
    (
        "desc_nomos8k",
        "Quick, clean sharpening for most photos. Needs the graphics card. Fast, about 8–13 seconds per small photo.",
        "Làm nét nhanh, sạch, hợp với đa số ảnh. Cần card đồ họa. Nhanh, khoảng 8–13 giây mỗi ảnh nhỏ.",
    ),
    (
        "desc_realesrgan",
        "A well-known classic style; makes photos clean and smooth but can look a bit painted. Needs the graphics card. Fast, about 8–13 seconds per small photo.",
        "Kiểu cổ điển quen thuộc; ảnh sạch, mịn nhưng đôi khi trông hơi giống tranh vẽ. Cần card đồ họa. Nhanh, khoảng 8–13 giây mỗi ảnh nhỏ.",
    ),
    (
        "desc_ultrasharp",
        "Makes edges and details very crisp. Personal use only, not for commercial use. Needs the graphics card. Fast, about 8–13 seconds per small photo.",
        "Làm đường nét và chi tiết rất sắc. Chỉ dùng cá nhân, không dùng thương mại. Cần card đồ họa. Nhanh, khoảng 8–13 giây mỗi ảnh nhỏ.",
    ),
    (
        "desc_scunet",
        "Removes grain and noise from dark or old photos without enlarging them; usually used as a step before sharpening.",
        "Khử nhiễu hạt cho ảnh chụp thiếu sáng hoặc ảnh cũ, không phóng to ảnh; thường dùng làm bước trước khi làm nét.",
    ),
    (
        "desc_drct",
        "Temporarily disabled because it currently makes photos blurry. Please choose another style.",
        "Tạm thời tắt vì hiện đang làm ảnh bị mờ. Hãy chọn kiểu khác.",
    ),
    (
        "stage_enhancing",
        "Sharpening the photo…",
        "Đang làm nét ảnh…",
    ),
    (
        "stage_denoising",
        "Removing grain…",
        "Đang khử nhiễu…",
    ),
    (
        "stage_resizing",
        "Resizing…",
        "Đang đổi kích thước…",
    ),
    (
        "stage_saving",
        "Saving…",
        "Đang lưu ảnh…",
    ),
    (
        "stage_auto",
        "Picked style: {}",
        "Đã chọn kiểu: {}",
    ),
    (
        "stage_gpu_fallback",
        "The graphics card gave a blank result; retrying on the processor…",
        "Card đồ họa cho ảnh trống; đang làm lại bằng bộ xử lý…",
    ),
    (
        "technical_details",
        "Technical details",
        "Thông tin kỹ thuật",
    ),
    (
        "tech_model",
        "Model: {}",
        "Model: {}",
    ),
    (
        "tech_backend",
        "Engine: {}",
        "Bộ chạy: {}",
    ),
    (
        "tech_scale",
        "Native scale: {}×",
        "Tỉ lệ gốc của model: {}×",
    ),
    (
        "tech_license",
        "License: {}",
        "Giấy phép: {}",
    ),
    (
        "tech_auto",
        "People: 4xRealWebPhoto_v4_dat2 · others: 4xNomos2_otf_esrgan · Fast Auto: 4xNomos8kSC",
        "Có người: 4xRealWebPhoto_v4_dat2 · ảnh khác: 4xNomos2_otf_esrgan · Tự động nhanh: 4xNomos8kSC",
    ),
    (
        "desc_custom",
        "A style you added yourself.",
        "Kiểu do bạn tự thêm.",
    ),
    (
        "threads_hint",
        "Fewer threads keep the computer smooth for other work; more threads finish a little faster.",
        "Ít luồng thì máy vẫn mượt để làm việc khác; nhiều luồng thì xong nhanh hơn một chút.",
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

/// Plain-language status for an engine stage line (the raw line goes to the log).
pub fn plain_stage(lang: Lang, raw: &str) -> String {
    let s = raw.trim();
    if let Some(rest) = s.strip_prefix("Auto chose ") {
        let name = rest.split(' ').next().unwrap_or(rest);
        let shown = model_display_name(lang, name).unwrap_or_else(|| name.to_string());
        return tr(lang, "stage_auto", &[&shown]);
    }
    if s.starts_with("Denoising") {
        return t(lang, "stage_denoising").to_string();
    }
    if s.starts_with("Enhancing") || s.starts_with("Upscaling with") {
        return t(lang, "stage_enhancing").to_string();
    }
    if s.starts_with("Lanczos3") {
        return t(lang, "stage_resizing").to_string();
    }
    if s.starts_with("Saving output") {
        return t(lang, "stage_saving").to_string();
    }
    if s.starts_with("NCNN output was black") {
        return t(lang, "stage_gpu_fallback").to_string();
    }
    localize_runtime(lang, raw)
}

pub fn model_description(lang: Lang, name: &str) -> Option<&'static str> {
    let key = match name {
        "4xNomos2_otf_esrgan" => "desc_nomos2",
        "4xRealWebPhoto_v4_dat2" => "desc_dat2",
        "spanx4_ch48" => "desc_span",
        "4xNomos8kSC" => "desc_nomos8k",
        "realesrgan-x4plus" => "desc_realesrgan",
        "ultrasharp-4x" => "desc_ultrasharp",
        "scunet_color_real_psnr" => "desc_scunet",
        "4xNomos2_hq_drct-l" => "desc_drct",
        _ => return None,
    };
    Some(t(lang, key))
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

    if let Some(value) = take_prefix(trimmed, &["Enhancing ", "Upscaling ", "Đang xử lý "]) {
        if value.ends_with('%') {
            return tr(lang, "upscaling_pct", &[value.trim_end_matches('%')]);
        }
    }
    if let Some(rest) = take_prefix(
        trimmed,
        &[
            "Saved the file, but the preview could not be shown: ",
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
            "Could not read the photo size: ",
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
        .filter(|_| true)
    {
        if [
            format!("{n} style(s)"),
            format!("{n} kiểu"),
            format!("{n} model(s)"),
        ]
        .contains(&trimmed.to_string())
        {
            return tr(lang, "model_count", &[&n.to_string()]);
        }
    }
    if let Some(rest) = take_prefix(trimmed, &["Disabled: ", "Đã tắt: ", "BROKEN: ", "Hỏng: "])
    {
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
            "Enhancing 42.5%",
            "Saved /tmp/out.png (800×600)",
            "Loaded 1920×1080",
            "Queued 12 images",
            "3 style(s)",
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
    fn every_model_and_auto_has_plain_description() {
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
        let jargon = [
            "ONNX",
            "PSNR",
            "checkpoint",
            "opset",
            "spandrel",
            "tile",
            "Lanczos",
            "NCNN",
            "Vulkan",
        ];
        let mut descs: Vec<&str> = vec![t(Lang::En, "desc_auto"), t(Lang::Vi, "desc_auto")];
        for name in names {
            for lang in Lang::all() {
                let d = model_description(lang, name).expect(name);
                assert!(!d.is_empty() && d != "[?]", "{name}");
                descs.push(d);
            }
        }
        for d in descs {
            for j in jargon {
                assert!(
                    !d.to_lowercase().contains(&j.to_lowercase()),
                    "jargon {j} in {d}"
                );
            }
        }
        assert!(t(Lang::Vi, "desc_ultrasharp").contains("không dùng thương mại"));
        assert!(t(Lang::Vi, "desc_drct").contains("tắt"));
        assert!(model_description(Lang::Vi, "unknown").is_none());
    }

    #[test]
    fn gui_keys_resolve_in_both_languages() {
        for key in [
            "desc_auto",
            "desc_custom",
            "stage_enhancing",
            "stage_saving",
            "stage_auto",
            "technical_details",
            "tech_model",
            "tech_backend",
            "tech_scale",
            "tech_license",
            "tech_auto",
            "threads_hint",
            "model",
            "gpu",
            "tile",
            "tile_hint",
            "cpu_hint",
            "tta",
            "tta_onnx_hint",
            "hint_keep_size",
            "hint_native",
            "hint_rescale",
            "onnx_threads",
        ] {
            assert_ne!(t(Lang::En, key), "[?]", "{key}");
            assert_ne!(t(Lang::Vi, key), "[?]", "{key}");
        }
    }

    #[test]
    fn engine_stages_become_plain_text() {
        let s = plain_stage(
            Lang::Vi,
            "Enhancing with 4xNomos2_otf_esrgan (ONNX Runtime CPU, 4 threads, tile 128, overlap 16)",
        );
        assert_eq!(s, "Đang làm nét ảnh…");
        assert_eq!(
            plain_stage(Lang::Vi, "Auto chose 4xNomos2_otf_esrgan (general)"),
            "Đã chọn kiểu: Động vật & chung, chi tiết nhất (Nomos2 OTF)"
        );
    }

    #[test]
    fn default_lang_is_vietnamese() {
        assert_eq!(Lang::default(), Lang::Vi);
    }
}
