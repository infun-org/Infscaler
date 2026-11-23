use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

use eframe::egui;

use crate::engine::{self, EngineEvent, GpuChoice, GpuDevice, JobOptions, RunOutcome};
use crate::models::{self, ModelInfo};
use crate::paths;

pub fn run() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("InfScaler")
            .with_inner_size([1240.0, 800.0])
            .with_min_inner_size([880.0, 560.0])
            .with_drag_and_drop(true),
        ..Default::default()
    };
    eframe::run_native(
        "InfScaler",
        options,
        Box::new(|cc| Ok(Box::new(InfScalerApp::new(cc)))),
    )
}

enum WorkerMsg {
    Event(EngineEvent),
    Finished(Result<RunOutcome, String>),
}

struct InfScalerApp {
    root: Option<PathBuf>,
    models: Vec<ModelInfo>,
    model_idx: usize,
    scale: u32,
    tile: u32,
    overlap: u32,
    gpu: String,
    gpus: Vec<GpuDevice>,
    tta: bool,
    keep_size: bool,
    denoise: bool,
    input_path: String,
    output_path: String,
    input_size: Option<(u32, u32)>,
    output_size: Option<(u32, u32)>,
    status: String,
    error: String,
    log: String,
    progress: f32,
    running: bool,
    started: Option<Instant>,
    elapsed: Option<Duration>,
    split: f32,
    before: Option<egui::TextureHandle>,
    after: Option<egui::TextureHandle>,
    rx: Option<Receiver<WorkerMsg>>,
    gpu_rx: Option<Receiver<Vec<GpuDevice>>>,
    cancel: Option<Arc<AtomicBool>>,
    pid: Option<Arc<AtomicU32>>,
    hovering_file: bool,
}

impl InfScalerApp {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        apply_style(&cc.egui_ctx);
        let root = paths::find_root();
        let mut app = Self {
            root,
            models: Vec::new(),
            model_idx: 0,
            scale: 4,
            tile: 0,
            overlap: 16,
            gpu: "auto".into(),
            gpus: Vec::new(),
            tta: false,
            keep_size: false,
            denoise: false,
            input_path: String::new(),
            output_path: String::new(),
            input_size: None,
            output_size: None,
            status: "Drop a photo, or open one.".into(),
            error: String::new(),
            log: String::new(),
            progress: 0.0,
            running: false,
            started: None,
            elapsed: None,
            split: 0.5,
            before: None,
            after: None,
            rx: None,
            gpu_rx: None,
            cancel: None,
            pid: None,
            hovering_file: false,
        };
        app.reload_models();
        app.spawn_gpu_probe();
        if app.root.is_none() {
            app.error = "Could not find the InfScaler folder (scripts/setup.sh). Start the app from the project directory, or set INFSCALER_HOME.".into();
        } else if app.models.is_empty() {
            app.status = "No models yet. Run scripts/setup.sh, then Reload.".into();
        }
        app
    }

    fn reload_models(&mut self) {
        let Some(root) = self.root.clone() else {
            self.models.clear();
            return;
        };
        let previous = self.models.get(self.model_idx).map(|m| m.name.clone());
        self.models = models::load_models(&root);
        self.model_idx = previous
            .and_then(|name| self.models.iter().position(|m| m.name == name))
            .filter(|idx| !self.models[*idx].is_broken())
            .or_else(|| {
                self.models
                    .iter()
                    .position(|m| m.name == "4xNomos2_otf_esrgan" && !m.is_broken())
            })
            .or_else(|| {
                self.models
                    .iter()
                    .position(|m| m.name == "4xNomos8kSC" && !m.is_broken())
            })
            .or_else(|| {
                self.models
                    .iter()
                    .position(|m| m.name == "realesrgan-x4plus" && !m.is_broken())
            })
            .or_else(|| self.models.iter().position(|m| !m.is_broken()))
            .unwrap_or(0);
    }

    fn spawn_gpu_probe(&mut self) {
        let (tx, rx) = mpsc::channel();
        self.gpu_rx = Some(rx);
        thread::spawn(move || {
            let _ = tx.send(engine::list_gpus());
        });
    }

    fn selected(&self) -> Option<&ModelInfo> {
        self.models.get(self.model_idx)
    }

    fn poll(&mut self, ctx: &egui::Context) {
        if let Some(rx) = &self.gpu_rx {
            match rx.try_recv() {
                Ok(gpus) => self.gpus = gpus,
                Err(TryRecvError::Empty) => {
                    ctx.request_repaint_after(Duration::from_millis(200));
                }
                Err(TryRecvError::Disconnected) => {}
            }
        }
        let Some(rx) = &self.rx else { return };
        loop {
            match rx.try_recv() {
                Ok(WorkerMsg::Event(EngineEvent::Percent(p))) => {
                    self.progress = p;
                    self.status = format!("Upscaling {p:.1}%");
                }
                Ok(WorkerMsg::Event(EngineEvent::Stage(text))) => {
                    self.status = text;
                }
                Ok(WorkerMsg::Event(EngineEvent::Log(line))) => {
                    push_log(&mut self.log, &line);
                }
                Ok(WorkerMsg::Finished(Ok(outcome))) => {
                    self.running = false;
                    self.elapsed = Some(outcome.elapsed);
                    self.progress = 100.0;
                    self.output_size = Some(outcome.output_size);
                    self.error.clear();
                    self.status = format!(
                        "Saved {} ({}×{})",
                        outcome.output.display(),
                        outcome.output_size.0,
                        outcome.output_size.1
                    );
                    match engine::make_preview(&outcome.output, 1600) {
                        Ok((rgba, w, h)) => {
                            self.after = Some(load_texture(ctx, "after", &rgba, w, h));
                        }
                        Err(err) => {
                            self.error = format!("Saved the file, but the preview failed: {err:#}");
                        }
                    }
                }
                Ok(WorkerMsg::Finished(Err(message))) => {
                    self.running = false;
                    if let Some(start) = self.started {
                        self.elapsed = Some(start.elapsed());
                    }
                    self.status = "Failed".into();
                    self.error = message;
                }
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => {
                    if self.running {
                        self.running = false;
                        self.status = "Failed".into();
                        self.error = "The processing thread exited unexpectedly.".into();
                    }
                    break;
                }
            }
        }
        if self.running {
            ctx.request_repaint();
        }
    }

    fn load_input(&mut self, ctx: &egui::Context, path: PathBuf) {
        self.error.clear();
        match engine::make_preview(&path, 1600) {
            Ok((rgba, w, h)) => {
                self.before = Some(load_texture(ctx, "before", &rgba, w, h));
                self.after = None;
                self.output_size = None;
                self.progress = 0.0;
                self.elapsed = None;
                self.split = 0.5;
                match image::image_dimensions(&path) {
                    Ok(size) => self.input_size = Some(size),
                    Err(err) => {
                        self.input_size = None;
                        self.error = format!("Could not read image size: {err}");
                    }
                }
                self.output_path = engine::default_output_path(&path).display().to_string();
                self.input_path = path.display().to_string();
                self.status = match self.input_size {
                    Some((w, h)) => format!("Loaded {w}×{h}"),
                    None => "Loaded".into(),
                };
            }
            Err(err) => {
                self.error = format!("{err:#}");
            }
        }
    }

    fn pick_input(&mut self, ctx: &egui::Context) {
        let picked = rfd::FileDialog::new()
            .set_title("Open photo")
            .add_filter("Images", &["png", "jpg", "jpeg", "webp"])
            .pick_file();
        if let Some(path) = picked {
            self.load_input(ctx, path);
        }
    }

    fn pick_output(&mut self) {
        let mut dialog = rfd::FileDialog::new()
            .set_title("Save enhanced image")
            .add_filter("PNG", &["png"])
            .add_filter("JPEG", &["jpg", "jpeg"])
            .add_filter("WebP", &["webp"]);
        if let Some(name) = Path::new(&self.output_path).file_name() {
            dialog = dialog.set_file_name(name.to_string_lossy());
        }
        if let Some(path) = dialog.save_file() {
            self.output_path = path.display().to_string();
        }
    }

    fn start_job(&mut self) {
        self.error.clear();
        let Some(root) = self.root.clone() else {
            self.error = "Project folder not found.".into();
            return;
        };
        let Some(model) = self.selected().cloned() else {
            self.error = "No model selected. Run scripts/setup.sh.".into();
            return;
        };
        if !model.files_present() {
            self.error = format!(
                "Model files for {} are missing ({}).",
                model.name,
                model.location()
            );
            return;
        }
        let input = PathBuf::from(self.input_path.trim());
        let output = PathBuf::from(self.output_path.trim());
        if input.as_os_str().is_empty() || !input.is_file() {
            self.error = "Choose an input image first.".into();
            return;
        }
        if output.as_os_str().is_empty() {
            self.error = "Choose an output path.".into();
            return;
        }
        let gpu = match GpuChoice::parse(&self.gpu) {
            Ok(gpu) => gpu,
            Err(err) => {
                self.error = err.to_string();
                return;
            }
        };
        if self.tile != 0 && self.tile < 32 {
            self.error = "Tile size must be 0 (auto) or at least 32.".into();
            return;
        }

        let opts = JobOptions {
            input,
            output,
            model: model.name.clone(),
            output_scale: self.scale,
            tile: self.tile,
            tta: self.tta && !model.is_onnx(),
            keep_size: self.keep_size,
            denoise: self.denoise,
            overlap: self.overlap,
            gpu,
        };
        let cancel = Arc::new(AtomicBool::new(false));
        let pid = Arc::new(AtomicU32::new(0));
        let (tx, rx) = mpsc::channel();
        let cancel_worker = cancel.clone();
        let pid_worker = pid.clone();
        self.cancel = Some(cancel);
        self.pid = Some(pid);
        self.rx = Some(rx);
        self.running = true;
        self.progress = 0.0;
        self.started = Some(Instant::now());
        self.elapsed = None;
        self.after = None;
        self.log.clear();
        self.status = "Starting…".into();

        thread::spawn(move || {
            let tx_events = tx.clone();
            let result = engine::run_job(&root, &opts, &cancel_worker, &pid_worker, move |event| {
                let _ = tx_events.send(WorkerMsg::Event(event));
            });
            let message = match result {
                Ok(outcome) => WorkerMsg::Finished(Ok(outcome)),
                Err(err) => WorkerMsg::Finished(Err(format!("{err:#}"))),
            };
            let _ = tx.send(message);
        });
    }

    fn cancel_job(&mut self) {
        if let Some(flag) = &self.cancel {
            flag.store(true, Ordering::SeqCst);
        }
        if let Some(pid) = &self.pid {
            let id = pid.load(Ordering::SeqCst);
            if id != 0 {
                let _ = std::process::Command::new("kill")
                    .args(["-TERM", &id.to_string()])
                    .status();
            }
        }
        self.status = "Cancelling…".into();
    }
}

impl eframe::App for InfScalerApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        self.poll(&ctx);
        self.hovering_file = ctx.input(|i| !i.raw.hovered_files.is_empty());

        let dropped: Vec<PathBuf> = ctx.input(|i| {
            i.raw
                .dropped_files
                .iter()
                .filter_map(|file| {
                    let path = file.path();
                    if engine::is_image_path(path) {
                        Some(path.to_path_buf())
                    } else {
                        None
                    }
                })
                .collect()
        });
        if let Some(path) = dropped.into_iter().next() {
            if !self.running {
                self.load_input(&ctx, path);
            }
        }
        if ctx.input(|i| i.modifiers.command && i.key_pressed(egui::Key::O)) && !self.running {
            self.pick_input(&ctx);
        }

        egui::Panel::top("header").show(ui, |ui| {
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                ui.heading("InfScaler");
                ui.label(
                    egui::RichText::new("local photo enhancement")
                        .weak()
                        .size(14.0),
                );
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("Reload models").clicked() && !self.running {
                        self.reload_models();
                        self.status = format!("{} model(s)", self.models.len());
                    }
                });
            });
            ui.add_space(4.0);
        });

        egui::Panel::bottom("status").show(ui, |ui| {
            ui.add_space(4.0);
            ui.add(egui::ProgressBar::new(self.progress / 100.0).show_percentage());
            ui.horizontal(|ui| {
                ui.label(&self.status);
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.monospace(format_elapsed(self.current_elapsed()));
                });
            });
            if !self.error.is_empty() {
                ui.colored_label(
                    egui::Color32::from_rgb(255, 140, 130),
                    egui::RichText::new(&self.error).size(13.0),
                );
            }
            if !self.log.is_empty() {
                egui::ScrollArea::vertical()
                    .max_height(72.0)
                    .stick_to_bottom(true)
                    .show(ui, |ui| {
                        ui.label(egui::RichText::new(&self.log).size(11.0).weak().monospace());
                    });
            }
            ui.add_space(4.0);
        });

        egui::Panel::left("controls")
            .resizable(true)
            .default_size(330.0)
            .show(ui, |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    self.controls(ui, &ctx);
                });
            });

        egui::CentralPanel::default().show(ui, |ui| {
            self.preview(ui);
        });
    }
}

impl InfScalerApp {
    fn controls(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        ui.add_space(4.0);
        ui.label("Photo");
        ui.horizontal(|ui| {
            if ui
                .add_enabled(!self.running, egui::Button::new("Open…"))
                .clicked()
            {
                self.pick_input(ctx);
            }
            ui.label(
                egui::RichText::new("or drop a file on the window")
                    .weak()
                    .size(12.0),
            );
        });
        ui.add_enabled_ui(!self.running, |ui| {
            ui.add(
                egui::TextEdit::singleline(&mut self.input_path)
                    .hint_text("input.jpg")
                    .desired_width(f32::INFINITY),
            );
        });
        if let Some((w, h)) = self.input_size {
            ui.label(egui::RichText::new(format!("{w} × {h} px")).weak());
        }

        ui.add_space(8.0);
        ui.label("Model");
        let onnx = self.selected().is_some_and(|m| m.is_onnx());
        let selected_name = self
            .selected()
            .map(|m| {
                if m.is_broken() {
                    format!("{} · BROKEN · {}×", m.name, m.scale)
                } else {
                    format!("{} · {} · {}×", m.name, m.kind().label(), m.scale)
                }
            })
            .unwrap_or_else(|| "none".into());
        egui::ComboBox::from_id_salt("model")
            .selected_text(selected_name)
            .width(ui.available_width())
            .show_ui(ui, |ui| {
                for (idx, model) in self.models.iter().enumerate() {
                    let label = if model.is_broken() {
                        format!("{} · BROKEN · {}×", model.name, model.scale)
                    } else {
                        format!(
                            "{} · {} · {}×",
                            model.name,
                            model.kind().label(),
                            model.scale
                        )
                    };
                    ui.add_enabled_ui(!model.is_broken(), |ui| {
                        ui.selectable_value(&mut self.model_idx, idx, label);
                    });
                }
                if self.models.is_empty() {
                    ui.label("No .param + .bin or .onnx models in models/");
                }
            });
        if let Some(model) = self.selected() {
            if let Some(reason) = &model.broken_reason {
                ui.label(
                    egui::RichText::new(format!("BROKEN: {reason}"))
                        .color(egui::Color32::from_rgb(220, 90, 70)),
                );
            }
            ui.label(
                egui::RichText::new(format!(
                    "{} · {}×  ·  {}",
                    model.kind().label(),
                    model.scale,
                    model.license
                ))
                .weak()
                .size(12.0),
            );
            if !model.notes.is_empty() {
                ui.label(egui::RichText::new(&model.notes).weak().size(12.0));
            }
            if !model.source.is_empty() {
                ui.label(egui::RichText::new(&model.source).weak().size(11.0));
            }
        }

        ui.add_space(8.0);
        ui.label("Output scale");
        ui.horizontal(|ui| {
            for value in [2, 3, 4] {
                ui.add_enabled_ui(!self.running && !self.keep_size, |ui| {
                    ui.selectable_value(&mut self.scale, value, format!("{value}×"));
                });
            }
        });
        let model_scale = self.selected().map(|m| m.scale).unwrap_or(4);
        let hint = if self.keep_size {
            format!("Runs the {model_scale}× model, then Lanczos3 back to the original pixels.")
        } else if self.scale == model_scale {
            format!("Writes the {model_scale}× model output.")
        } else {
            format!(
                "Runs the {model_scale}× model, then Lanczos3 to {0}× the original.",
                self.scale
            )
        };
        ui.label(egui::RichText::new(hint).weak().size(12.0));

        ui.add_space(8.0);
        ui.horizontal(|ui| {
            ui.label("Tile");
            ui.add_enabled(
                !self.running,
                egui::DragValue::new(&mut self.tile)
                    .range(0..=1024)
                    .speed(16),
            );
            ui.label(egui::RichText::new("0 = auto").weak());
        });
        ui.horizontal(|ui| {
            ui.label("Overlap");
            ui.add_enabled(
                !self.running,
                egui::DragValue::new(&mut self.overlap)
                    .range(0..=256)
                    .speed(4),
            );
            ui.label(egui::RichText::new("ONNX seams").weak());
        });
        ui.label(
            egui::RichText::new(
                "NCNN uses the Vulkan tile. ONNX blends a Hann ramp on the CPU. DAT, DRCT, and SCUNet use their fixed input size and at least 48 px of overlap.",
            )
            .weak()
            .size(12.0),
        );

        ui.add_space(8.0);
        ui.label("GPU");
        let gpu_label = self.gpu_label();
        egui::ComboBox::from_id_salt("gpu")
            .selected_text(gpu_label)
            .width(ui.available_width())
            .show_ui(ui, |ui| {
                ui.selectable_value(&mut self.gpu, "auto".into(), "Auto");
                for device in &self.gpus {
                    ui.selectable_value(&mut self.gpu, device.id.clone(), &device.label);
                }
                ui.label("CPU runs ONNX models. NCNN binaries reject -g -1.");
            });

        ui.add_space(6.0);
        ui.add_enabled(
            !self.running && !onnx,
            egui::Checkbox::new(&mut self.tta, "TTA (8 flips, slower)"),
        );
        if onnx {
            ui.label(
                egui::RichText::new(
                    "This model runs on the CPU through ONNX Runtime. TTA stays off.",
                )
                .weak()
                .size(12.0),
            );
        }
        ui.add_enabled(
            !self.running,
            egui::Checkbox::new(&mut self.keep_size, "Keep original size"),
        );
        let has_denoise = self.models.iter().any(|m| m.denoise);
        ui.add_enabled(
            !self.running && has_denoise,
            egui::Checkbox::new(&mut self.denoise, "Denoise first (SCUNet)"),
        );

        ui.add_space(8.0);
        ui.label("Save as");
        ui.horizontal(|ui| {
            ui.add_enabled_ui(!self.running, |ui| {
                ui.add(
                    egui::TextEdit::singleline(&mut self.output_path)
                        .hint_text("enhanced.png")
                        .desired_width(ui.available_width() - 78.0),
                );
            });
            if ui
                .add_enabled(!self.running, egui::Button::new("Browse"))
                .clicked()
            {
                self.pick_output();
            }
        });
        if let Some((w, h)) = self.output_size {
            ui.label(egui::RichText::new(format!("Last result {w} × {h} px")).weak());
        }

        ui.add_space(12.0);
        if self.running {
            if ui
                .add_sized(
                    [ui.available_width(), 36.0],
                    egui::Button::new("Cancel").fill(egui::Color32::from_rgb(130, 48, 48)),
                )
                .clicked()
            {
                self.cancel_job();
            }
        } else {
            let ready = !self.input_path.trim().is_empty()
                && self.selected().is_some_and(|model| !model.is_broken());
            let mut enhance = false;
            ui.add_enabled_ui(ready, |ui| {
                if ui
                    .add_sized(
                        [ui.available_width(), 36.0],
                        egui::Button::new("Enhance").fill(egui::Color32::from_rgb(32, 110, 186)),
                    )
                    .clicked()
                {
                    enhance = true;
                }
            });
            if enhance {
                self.start_job();
            }
        }
        ui.add_space(8.0);
    }

    fn preview(&mut self, ui: &mut egui::Ui) {
        let available = ui.available_size();
        let (rect, response) = ui.allocate_exact_size(available, egui::Sense::click_and_drag());
        let before_size = self.before.as_ref().map(|tex| tex.size_vec2());
        let before_id = self.before.as_ref().map(|tex| tex.id());
        let after_id = self.after.as_ref().map(|tex| tex.id());

        let painter = ui.painter_at(rect);
        painter.rect_filled(rect, 0.0, egui::Color32::from_rgb(18, 18, 20));

        let Some(before_size) = before_size else {
            painter.text(
                rect.center(),
                egui::Align2::CENTER_CENTER,
                "Drop a photo of a person or animal here",
                egui::FontId::proportional(18.0),
                egui::Color32::from_rgb(170, 170, 176),
            );
            if self.hovering_file {
                painter.rect_stroke(
                    rect.shrink(8.0),
                    6.0,
                    egui::Stroke::new(2.0, egui::Color32::from_rgb(80, 150, 230)),
                    egui::StrokeKind::Inside,
                );
            }
            return;
        };
        let Some(before_id) = before_id else { return };

        let aspect = if before_size.y <= 0.0 {
            1.0
        } else {
            before_size.x / before_size.y
        };
        let fitted = fit_rect(rect.shrink(16.0), aspect);

        if response.dragged() {
            if let Some(pos) = response.interact_pointer_pos() {
                self.split = ((pos.x - fitted.left()) / fitted.width()).clamp(0.02, 0.98);
            }
        }

        let uv = egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0));
        if let Some(after_id) = after_id {
            painter.image(after_id, fitted, uv, egui::Color32::WHITE);
            let clip = egui::Rect::from_min_max(
                fitted.min,
                egui::pos2(fitted.left() + fitted.width() * self.split, fitted.bottom()),
            );
            painter
                .with_clip_rect(clip)
                .image(before_id, fitted, uv, egui::Color32::WHITE);
            let x = fitted.left() + fitted.width() * self.split;
            painter.line_segment(
                [egui::pos2(x, fitted.top()), egui::pos2(x, fitted.bottom())],
                egui::Stroke::new(2.0, egui::Color32::WHITE),
            );
            painter.circle_filled(egui::pos2(x, fitted.center().y), 7.0, egui::Color32::WHITE);
            paint_tag(
                &painter,
                egui::pos2(fitted.left() + 8.0, fitted.top() + 8.0),
                "Before",
            );
            paint_tag(
                &painter,
                egui::pos2(fitted.right() - 64.0, fitted.top() + 8.0),
                "After",
            );
        } else {
            painter.image(before_id, fitted, uv, egui::Color32::WHITE);
            paint_tag(
                &painter,
                egui::pos2(fitted.left() + 8.0, fitted.top() + 8.0),
                "Original",
            );
        }

        if self.hovering_file {
            painter.rect_stroke(
                rect.shrink(6.0),
                6.0,
                egui::Stroke::new(2.0, egui::Color32::from_rgb(80, 150, 230)),
                egui::StrokeKind::Inside,
            );
        }
    }

    fn gpu_label(&self) -> String {
        if self.gpu == "auto" {
            return "Auto".into();
        }
        if self.gpu == "cpu" {
            return "CPU".into();
        }
        self.gpus
            .iter()
            .find(|g| g.id == self.gpu)
            .map(|g| g.label.clone())
            .unwrap_or_else(|| format!("GPU {}", self.gpu))
    }

    fn current_elapsed(&self) -> Option<Duration> {
        if self.running {
            self.started.map(|t| t.elapsed())
        } else {
            self.elapsed
        }
    }
}

fn apply_style(ctx: &egui::Context) {
    let mut visuals = egui::Visuals::dark();
    visuals.selection.bg_fill = egui::Color32::from_rgb(36, 102, 176);
    visuals.widgets.active.bg_fill = egui::Color32::from_rgb(40, 110, 180);
    visuals.hyperlink_color = egui::Color32::from_rgb(130, 186, 255);
    ctx.set_visuals(visuals);
    ctx.style_mut_of(ctx.theme(), |style| {
        style.spacing.item_spacing = egui::vec2(8.0, 6.0);
        style.spacing.button_padding = egui::vec2(10.0, 5.0);
    });
}

fn load_texture(
    ctx: &egui::Context,
    name: &str,
    rgba: &[u8],
    width: u32,
    height: u32,
) -> egui::TextureHandle {
    ctx.load_texture(
        name,
        egui::ColorImage::from_rgba_unmultiplied([width as usize, height as usize], rgba),
        egui::TextureOptions::LINEAR,
    )
}

fn fit_rect(bounds: egui::Rect, aspect: f32) -> egui::Rect {
    let bw = bounds.width().max(1.0);
    let bh = bounds.height().max(1.0);
    let bounds_aspect = bw / bh;
    let (w, h) = if aspect > bounds_aspect {
        (bw, bw / aspect.max(0.01))
    } else {
        (bh * aspect, bh)
    };
    let x = bounds.left() + (bw - w) * 0.5;
    let y = bounds.top() + (bh - h) * 0.5;
    egui::Rect::from_min_size(egui::pos2(x, y), egui::vec2(w, h))
}

fn paint_tag(painter: &egui::Painter, pos: egui::Pos2, text: &str) {
    let galley = painter.layout_no_wrap(
        text.to_string(),
        egui::FontId::proportional(13.0),
        egui::Color32::WHITE,
    );
    let rect = egui::Rect::from_min_size(pos, galley.size() + egui::vec2(12.0, 6.0));
    painter.rect_filled(rect, 4.0, egui::Color32::from_black_alpha(160));
    painter.galley(pos + egui::vec2(6.0, 3.0), galley, egui::Color32::WHITE);
}

fn push_log(log: &mut String, line: &str) {
    if log.len() > 8_000 {
        let mut cut = log.len() - 6_000;
        while cut < log.len() && !log.is_char_boundary(cut) {
            cut += 1;
        }
        *log = log[cut..].to_string();
    }
    log.push_str(line);
    log.push('\n');
}

fn format_elapsed(elapsed: Option<Duration>) -> String {
    let Some(elapsed) = elapsed else {
        return "00:00.0".into();
    };
    let total_ms = elapsed.as_millis();
    let minutes = total_ms / 60_000;
    let seconds = (total_ms / 1000) % 60;
    let tenths = (total_ms % 1000) / 100;
    format!("{minutes:02}:{seconds:02}.{tenths}")
}
