use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use eframe::egui;

use crate::engine::{self, EngineEvent, GpuChoice, GpuDevice, JobOptions, RunOutcome};
use crate::i18n::{self, Lang};
use crate::models::{self, ModelInfo};
use crate::paths;
use crate::theme;

pub struct ScreenshotSpec {
    pub output: PathBuf,
    pub input: Option<PathBuf>,
    pub after: Option<PathBuf>,
    pub lang: Option<Lang>,
    pub expand: bool,
    pub scroll: bool,
}

struct ScreenshotJob {
    output: PathBuf,
    frames: u32,
    requested: bool,
    done: Arc<AtomicBool>,
    error: Arc<Mutex<Option<String>>>,
}

pub fn run() -> eframe::Result {
    run_inner(None)
}

pub fn run_screenshot(spec: ScreenshotSpec) -> eframe::Result {
    run_inner(Some(spec))
}

fn prefer_x11_for_drag_and_drop() {
    // winit's Wayland backend does not deliver file drag-and-drop events, so
    // prefer X11 (XWayland) when it is available. Set INFSCALER_WAYLAND=1 to
    // keep native Wayland.
    if std::env::var_os("DISPLAY").is_some() && std::env::var_os("INFSCALER_WAYLAND").is_none() {
        // SAFETY: called at startup before any other threads are spawned.
        unsafe { std::env::remove_var("WAYLAND_DISPLAY") };
    }
}

fn window_icon() -> egui::IconData {
    let image = image::load_from_memory(include_bytes!("../assets/icon/infscaler-256.png"))
        .expect("embedded window icon")
        .into_rgba8();
    let (width, height) = image.dimensions();
    egui::IconData {
        rgba: image.into_raw(),
        width,
        height,
    }
}

fn run_inner(screenshot: Option<ScreenshotSpec>) -> eframe::Result {
    prefer_x11_for_drag_and_drop();
    let size = if screenshot.is_some() {
        [1400.0, 1120.0]
    } else {
        [1280.0, 840.0]
    };
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("InfScaler")
            .with_app_id("infscaler")
            .with_icon(window_icon())
            .with_inner_size(size)
            .with_min_inner_size([880.0, 560.0])
            .with_drag_and_drop(true),
        ..Default::default()
    };
    eframe::run_native(
        "InfScaler",
        options,
        Box::new(move |cc| Ok(Box::new(InfScalerApp::new(cc, screenshot)))),
    )
}

enum WorkerMsg {
    Event(EngineEvent),
    Finished(Result<RunOutcome, String>),
    BatchFinished(Result<(), String>),
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
    face_restore: bool,
    face_strength: f32,
    fast: bool,
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
    dragging_split: bool,
    before: Option<egui::TextureHandle>,
    after: Option<egui::TextureHandle>,
    rx: Option<Receiver<WorkerMsg>>,
    gpu_rx: Option<Receiver<Vec<GpuDevice>>>,
    cancel: Option<Arc<AtomicBool>>,
    pid: Option<Arc<AtomicU32>>,
    hovering_file: bool,
    queued: Vec<PathBuf>,
    batch_jobs: usize,
    batch_threads: usize,
    batch_preset: String,
    batch_output_dir: Option<PathBuf>,
    batch_control: Option<Arc<crate::batch::BatchControl>>,
    lang: Lang,
    persist_lang: bool,
    expand_tech: bool,
    screenshot: Option<ScreenshotJob>,
    scroll_controls: bool,
    zoom: f32,
    pan: egui::Vec2,
}

impl InfScalerApp {
    fn new(cc: &eframe::CreationContext<'_>, screenshot: Option<ScreenshotSpec>) -> Self {
        theme::apply(&cc.egui_ctx);
        let persist_lang = screenshot.is_none();
        let expand_tech = screenshot.as_ref().is_some_and(|s| s.expand);
        let lang = screenshot
            .as_ref()
            .and_then(|s| s.lang)
            .unwrap_or_else(i18n::load_lang);
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
            keep_size: true,
            denoise: false,
            face_restore: false,
            face_strength: 0.8,
            fast: false,
            input_path: String::new(),
            output_path: String::new(),
            input_size: None,
            output_size: None,
            status: i18n::t(lang, "drop_or_open").to_string(),
            error: String::new(),
            log: String::new(),
            progress: 0.0,
            running: false,
            started: None,
            elapsed: None,
            split: 0.5,
            dragging_split: false,
            before: None,
            after: None,
            rx: None,
            gpu_rx: None,
            cancel: None,
            pid: None,
            hovering_file: false,
            queued: Vec::new(),
            batch_jobs: 1,
            batch_threads: 4,
            batch_preset: "general".into(),
            batch_output_dir: None,
            batch_control: None,
            lang,
            persist_lang,
            expand_tech,
            screenshot: None,
            scroll_controls: false,
            zoom: 1.0,
            pan: egui::Vec2::ZERO,
        };
        app.reload_models();
        app.spawn_gpu_probe();
        if app.root.is_none() {
            app.error = i18n::t(lang, "missing_project_folder_long").to_string();
        } else if app.models.is_empty() {
            app.status = i18n::t(lang, "no_models_yet").to_string();
        }
        if let Some(spec) = screenshot {
            app.prepare_screenshot(&cc.egui_ctx, spec);
        }
        app
    }

    fn prepare_screenshot(&mut self, ctx: &egui::Context, spec: ScreenshotSpec) {
        if spec.expand {
            if let Some(idx) = self
                .models
                .iter()
                .position(|m| m.name == "scunet_color_real_psnr")
                .or_else(|| self.models.iter().position(|m| !m.is_broken()))
            {
                self.model_idx = idx;
            }
        }
        if let Some(path) = spec.input {
            self.load_input(ctx, path);
        }
        if let Some(path) = spec.after {
            match engine::make_preview(&path, 1600) {
                Ok((rgba, w, h)) => {
                    self.after = Some(load_texture(ctx, "after", &rgba, w, h));
                    if let Ok(size) = image::image_dimensions(&path) {
                        self.output_size = Some(size);
                    }
                    self.output_path = path.display().to_string();
                    self.progress = 100.0;
                }
                Err(err) => self.error = format!("{err:#}"),
            }
        }
        self.scroll_controls = spec.scroll;
        self.screenshot = Some(ScreenshotJob {
            output: spec.output,
            frames: 0,
            requested: false,
            done: Arc::new(AtomicBool::new(false)),
            error: Arc::new(Mutex::new(None)),
        });
    }

    fn tick_screenshot(&mut self, ctx: &egui::Context) {
        let Some(job) = self.screenshot.as_mut() else {
            return;
        };
        job.frames += 1;
        ctx.request_repaint();
        if job.done.load(Ordering::SeqCst) {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            return;
        }
        if job.frames >= 10 && !job.requested {
            job.requested = true;
            let output = job.output.clone();
            let done = job.done.clone();
            let error = job.error.clone();
            ctx.request_screenshot(move |image| {
                if let Err(err) = theme::save_color_image(&image, &output) {
                    *error.lock().unwrap() = Some(err);
                }
                done.store(true, Ordering::SeqCst);
            });
        }
        if job.frames > 180 {
            *job.error.lock().unwrap() = Some("screenshot timed out".into());
            job.done.store(true, Ordering::SeqCst);
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
    }

    fn reload_models(&mut self) {
        let Some(root) = self.root.clone() else {
            self.models.clear();
            return;
        };
        let previous = self.models.get(self.model_idx).map(|m| m.name.clone());
        self.models = models::load_models(&root);
        if previous.is_none() {
            self.model_idx = self.models.len();
            return;
        }
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
                    self.status = self.tr("upscaling_pct", &[&format!("{p:.1}")]);
                }
                Ok(WorkerMsg::Event(EngineEvent::Stage(text))) => {
                    self.status = i18n::plain_stage(self.lang, &text);
                    push_log(&mut self.log, &text);
                }
                Ok(WorkerMsg::Event(EngineEvent::Log(line))) => {
                    if let Some((done, total)) = line
                        .split_whitespace()
                        .next()
                        .and_then(|ratio| ratio.split_once('/'))
                    {
                        if let (Ok(done), Ok(total)) = (done.parse::<f32>(), total.parse::<f32>()) {
                            if total > 0.0 {
                                self.progress = done / total * 100.0;
                                self.status = line.clone();
                            }
                        }
                    }
                    push_log(&mut self.log, &line);
                }
                Ok(WorkerMsg::Finished(Ok(outcome))) => {
                    self.running = false;
                    self.elapsed = Some(outcome.elapsed);
                    self.progress = 100.0;
                    self.output_size = Some(outcome.output_size);
                    self.error.clear();
                    self.status = self.tr(
                        "saved",
                        &[
                            &outcome.output.display().to_string(),
                            &outcome.output_size.0.to_string(),
                            &outcome.output_size.1.to_string(),
                        ],
                    );
                    match engine::make_preview(&outcome.output, 1600) {
                        Ok((rgba, w, h)) => {
                            self.after = Some(load_texture(ctx, "after", &rgba, w, h));
                        }
                        Err(err) => {
                            self.error = self.tr("saved_preview_failed", &[&format!("{err:#}")]);
                        }
                    }
                }
                Ok(WorkerMsg::Finished(Err(message))) => {
                    self.running = false;
                    if let Some(start) = self.started {
                        self.elapsed = Some(start.elapsed());
                    }
                    self.status = self.t("failed").to_string();
                    self.error = i18n::localize_runtime(self.lang, &message);
                }
                Ok(WorkerMsg::BatchFinished(result)) => {
                    self.running = false;
                    self.batch_control = None;
                    self.status = match result {
                        Ok(()) => self.t("batch_complete").to_string(),
                        Err(e) => self.tr("batch_stopped", &[&e]),
                    };
                }
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => {
                    if self.running {
                        self.running = false;
                        self.status = self.t("failed").to_string();
                        self.error = self.t("thread_exited").to_string();
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
                        self.error = self.tr("could_not_read_size", &[&err.to_string()]);
                    }
                }
                self.output_path = engine::default_output_path(&path).display().to_string();
                self.input_path = path.display().to_string();
                self.status = match self.input_size {
                    Some((w, h)) => self.tr("loaded_size", &[&w.to_string(), &h.to_string()]),
                    None => self.t("loaded").to_string(),
                };
            }
            Err(err) => {
                self.error = format!("{err:#}");
            }
        }
    }

    fn pick_input(&mut self, ctx: &egui::Context) {
        let picked = rfd::FileDialog::new()
            .set_title(self.t("open_photo"))
            .add_filter(self.t("images_filter"), &["png", "jpg", "jpeg", "webp"])
            .pick_file();
        if let Some(path) = picked {
            self.load_input(ctx, path);
        }
    }

    fn pick_output(&mut self) {
        let mut dialog = rfd::FileDialog::new()
            .set_title(self.t("save_enhanced"))
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
            self.error = self.t("project_folder_missing").to_string();
            return;
        };
        let automatic = self.model_idx == self.models.len();
        let model = self.selected().cloned();
        if !automatic && model.is_none() {
            self.error = self.t("no_model_selected").to_string();
            return;
        }
        if let Some(model) = model.as_ref().filter(|_| !automatic) {
            if !model.files_present() {
                self.error = self.tr("model_files_missing", &[&model.name, &model.location()]);
                return;
            }
        }
        let input = PathBuf::from(self.input_path.trim());
        let output = PathBuf::from(self.output_path.trim());
        if input.as_os_str().is_empty() || !input.is_file() {
            self.error = self.t("choose_input").to_string();
            return;
        }
        if output.as_os_str().is_empty() {
            self.error = self.t("choose_output").to_string();
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
            self.error = self.t("tile_size_invalid").to_string();
            return;
        }

        let opts = JobOptions {
            input,
            output,
            model: model
                .as_ref()
                .map(|m| m.name.clone())
                .unwrap_or_else(|| "auto".into()),
            output_scale: self.scale,
            tile: self.tile,
            tta: self.tta && model.as_ref().is_some_and(|m| !m.is_onnx()),
            keep_size: self.keep_size,
            denoise: self.denoise,
            overlap: self.overlap,
            gpu,
            face_restore: self.face_restore,
            face_strength: self.face_strength,
            face_model: "RestoreFormerPlusPlus".into(),
            fast: self.fast,
            preset: if self.fast {
                "fast".into()
            } else {
                self.batch_preset.clone()
            },
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
        self.status = self.t("starting").to_string();

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
        if let Some(control) = &self.batch_control {
            control.cancelled.store(true, Ordering::SeqCst);
        }
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
        self.status = self.t("cancelling").to_string();
    }

    fn start_batch(&mut self) {
        let Some(root) = self.root.clone() else {
            self.error = self.t("project_folder_missing").to_string();
            return;
        };
        let Some(first) = self.queued.first().cloned() else {
            return;
        };
        let input_dir = first.parent().unwrap_or(Path::new(".")).to_path_buf();
        let files = self.queued.clone();
        let control = Arc::new(crate::batch::BatchControl::default());
        self.batch_control = Some(control.clone());
        let (tx, rx) = mpsc::channel();
        self.rx = Some(rx);
        self.running = true;
        self.started = Some(Instant::now());
        self.progress = 0.0;
        self.error.clear();
        let opts = crate::batch::BatchOptions {
            input_dir,
            input_files: Some(files),
            output_dir: self.batch_output_dir.clone(),
            jobs: self.batch_jobs,
            threads: self.batch_threads,
            mode: if self.keep_size {
                crate::batch::OutputMode::Sharpen
            } else if self.scale == 2 {
                crate::batch::OutputMode::Upscale2
            } else if self.scale == 3 {
                crate::batch::OutputMode::Upscale3
            } else {
                crate::batch::OutputMode::Upscale4
            },
            preset: self.batch_preset.clone(),
        };
        thread::spawn(move || {
            let tx_report = tx.clone();
            let result = crate::batch::run(&root, &opts, control, move |line| {
                let _ = tx_report.send(WorkerMsg::Event(EngineEvent::Log(line)));
            });
            let _ = tx.send(WorkerMsg::BatchFinished(
                result.map_err(|e| format!("{e:#}")),
            ));
        });
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
                    if path.is_dir() {
                        Some(path.to_path_buf())
                    } else if engine::is_image_path(path) {
                        Some(path.to_path_buf())
                    } else {
                        None
                    }
                })
                .collect()
        });
        for path in dropped {
            if !self.running {
                if path.is_dir() {
                    if let Ok(files) = crate::batch::scan_folder(&path) {
                        self.queued.extend(files);
                    }
                } else {
                    self.queued.push(path.clone());
                    self.load_input(&ctx, path);
                }
            }
        }
        if ctx.input(|i| i.modifiers.command && i.key_pressed(egui::Key::O)) && !self.running {
            self.pick_input(&ctx);
        }

        egui::Panel::top("header")
            .frame(theme::header_panel())
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.add(egui::Label::new(theme::title_text("InfScaler")));
                    ui.add_space(12.0);
                    ui.label(theme::caption_text(self.t("tagline")));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui
                            .add_enabled(
                                !self.running,
                                theme::standard_button(self.t("reload_models")),
                            )
                            .clicked()
                        {
                            self.reload_models();
                            self.status = self.tr("model_count", &[&self.models.len().to_string()]);
                        }
                        ui.add_space(8.0);
                        theme::segmented_frame().show(ui, |ui| {
                            ui.spacing_mut().item_spacing.x = 0.0;
                            ui.horizontal(|ui| {
                                let current = self.lang;
                                for lang in Lang::all() {
                                    if ui
                                        .add(theme::segmented_button(lang.label(), current == lang))
                                        .clicked()
                                        && current != lang
                                    {
                                        self.set_lang(lang);
                                    }
                                }
                            });
                        });
                    });
                });
            });

        egui::Panel::bottom("status")
            .frame(theme::status_panel())
            .show(ui, |ui| {
                ui.scope(|ui| {
                    ui.spacing_mut().interact_size.y = 4.0;
                    ui.add(
                        egui::ProgressBar::new((self.progress / 100.0).clamp(0.0, 1.0))
                            .fill(theme::ACCENT)
                            .desired_width(f32::INFINITY),
                    );
                });
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new(&self.status)
                            .color(theme::TEXT)
                            .size(14.0),
                    );
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(
                            egui::RichText::new(format_elapsed(self.current_elapsed()))
                                .monospace()
                                .color(theme::TEXT_SECONDARY)
                                .size(12.0),
                        );
                    });
                });
                if !self.error.is_empty() {
                    ui.colored_label(theme::ERROR, egui::RichText::new(&self.error).size(13.0));
                }
                if !self.log.is_empty() {
                    egui::ScrollArea::vertical()
                        .max_height(72.0)
                        .stick_to_bottom(true)
                        .show(ui, |ui| {
                            ui.label(
                                egui::RichText::new(&self.log)
                                    .size(11.0)
                                    .color(theme::TEXT_SECONDARY)
                                    .monospace(),
                            );
                        });
                }
            });

        egui::Panel::left("controls")
            .resizable(true)
            .default_size(400.0)
            .min_size(320.0)
            .max_size(440.0)
            .show_separator_line(false)
            .frame(theme::mica_panel())
            .show(ui, |ui| {
                let height = ui.available_height();
                // Leave room for the card's inner margin, stroke and shadow so the
                // last controls stay inside the rounded card and can scroll.
                let scroll_h = (height - 48.0).max(64.0);
                theme::card_frame().show(ui, |ui| {
                    egui::ScrollArea::vertical()
                        .id_salt("controls-scroll")
                        .auto_shrink([false, false])
                        .max_height(scroll_h)
                        .show(ui, |ui| {
                            ui.set_width(ui.available_width());
                            self.controls(ui, &ctx);
                        });
                });
            });

        egui::CentralPanel::default()
            .frame(theme::mica_panel())
            .show(ui, |ui| {
                let available = ui.available_size();
                theme::card_frame().inner_margin(8.0).show(ui, |ui| {
                    ui.set_min_size(egui::vec2(
                        (available.x - 24.0).max(64.0),
                        (available.y - 24.0).max(64.0),
                    ));
                    self.preview(ui);
                });
            });

        self.tick_screenshot(&ctx);
    }
}

impl InfScalerApp {
    fn controls(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        ui.label(theme::subtitle_text(self.t("photo")));
        ui.add_space(8.0);
        ui.horizontal(|ui| {
            if ui
                .add_enabled(!self.running, theme::standard_button(self.t("open")))
                .clicked()
            {
                self.pick_input(ctx);
            }
            ui.label(theme::caption_text(self.t("or_drop_file")));
        });
        if ui
            .add_enabled(
                !self.running,
                theme::standard_button(
                    self.batch_output_dir
                        .as_ref()
                        .map(|p| self.tr("output_folder", &[&p.display().to_string()]))
                        .unwrap_or_else(|| self.t("output_folder_auto").to_string()),
                ),
            )
            .clicked()
        {
            if let Some(path) = rfd::FileDialog::new()
                .set_title(self.t("choose_output_folder"))
                .pick_folder()
            {
                self.batch_output_dir = Some(path);
            }
        }
        ui.horizontal(|ui| {
            ui.label(self.t("onnx_threads"))
                .on_hover_text(self.t("threads_hint"));
            ui.add(egui::DragValue::new(&mut self.batch_threads).range(1..=12));
        });
        if ui
            .add_enabled(!self.running, theme::standard_button(self.t("open_folder")))
            .clicked()
        {
            if let Some(folder) = rfd::FileDialog::new()
                .set_title(self.t("open_image_folder"))
                .pick_folder()
            {
                if let Ok(files) = crate::batch::scan_folder(&folder) {
                    self.queued = files;
                    self.status = self.tr("queued_top_level", &[&self.queued.len().to_string()]);
                }
            }
        }
        ui.label(self.tr("queue_count", &[&self.queued.len().to_string()]));
        egui::ComboBox::from_id_salt("batch-preset")
            .selected_text(i18n::preset_label(self.lang, &self.batch_preset))
            .width((ui.available_width() - 8.0).max(120.0))
            .show_ui(ui, |ui| {
                for p in ["general", "people", "fast"] {
                    ui.selectable_value(
                        &mut self.batch_preset,
                        p.into(),
                        i18n::preset_label(self.lang, p),
                    );
                }
            });
        theme::segmented_frame().show(ui, |ui| {
            ui.spacing_mut().item_spacing.x = 0.0;
            ui.horizontal(|ui| {
                for n in 1..=3 {
                    if ui
                        .add(theme::segmented_button(
                            i18n::jobs_label(self.lang, n),
                            self.batch_jobs == n,
                        ))
                        .clicked()
                        && !self.running
                    {
                        self.batch_jobs = n;
                    }
                }
            });
        });
        if !self.queued.is_empty() {
            egui::ScrollArea::vertical()
                .max_height(110.0)
                .show(ui, |ui| {
                    for path in self.queued.iter().take(80) {
                        ui.label(path.file_name().unwrap_or_default().to_string_lossy());
                    }
                    if self.queued.len() > 80 {
                        ui.label(self.tr("and_more", &[&(self.queued.len() - 80).to_string()]));
                    }
                });
            if ui
                .add_enabled(
                    !self.running,
                    theme::accent_button(self.t("process_queued")),
                )
                .clicked()
            {
                self.start_batch();
            }
        }
        ui.add_enabled_ui(!self.running, |ui| {
            let response = ui.add(
                egui::TextEdit::singleline(&mut self.input_path)
                    .hint_text("input.jpg")
                    .desired_width(f32::INFINITY),
            );
            theme::underline_focus(ui, &response);
        });
        if let Some((w, h)) = self.input_size {
            ui.label(theme::caption_text(format!("{w} × {h} px")));
        }

        ui.add_space(16.0);
        ui.label(theme::subtitle_text(self.t("model")));
        ui.add_space(8.0);
        let onnx = self.selected().is_some_and(|m| m.is_onnx());
        let selected_name = if self.model_idx == self.models.len() {
            self.t("auto_choose").to_string()
        } else {
            self.selected()
                .map(|m| m.display_name(self.lang))
                .unwrap_or_else(|| self.t("none").to_string())
        };
        egui::ComboBox::from_id_salt("model")
            .selected_text(selected_name)
            .width(ui.available_width())
            .show_ui(ui, |ui| {
                let auto_choose = self.t("auto_choose");
                ui.selectable_value(&mut self.model_idx, self.models.len(), auto_choose);
                for (idx, model) in self.models.iter().enumerate() {
                    let label = model.display_name(self.lang);
                    let tip = format!(
                        "{} · {} · {}×{}",
                        model.name,
                        model.kind().label(),
                        model.scale,
                        if model.is_broken() {
                            self.t("broken_suffix")
                        } else {
                            ""
                        }
                    );
                    ui.add_enabled_ui(!model.is_broken(), |ui| {
                        ui.selectable_value(&mut self.model_idx, idx, label)
                            .on_hover_text(tip);
                    });
                }
                if self.models.is_empty() {
                    ui.label(self.t("no_models_in_folder"));
                }
            });
        let lang = self.lang;
        if self.model_idx == self.models.len() {
            ui.label(egui::RichText::new(self.t("desc_auto")).size(12.5));
            egui::CollapsingHeader::new(self.t("technical_details"))
                .id_salt("tech-auto")
                .default_open(false)
                .open(if self.expand_tech { Some(true) } else { None })
                .show(ui, |ui| {
                    ui.label(
                        egui::RichText::new(i18n::t(lang, "tech_auto"))
                            .weak()
                            .size(11.5),
                    );
                });
        } else if let Some(model) = self.selected() {
            let desc = i18n::model_description(lang, &model.name)
                .unwrap_or_else(|| i18n::t(lang, "desc_custom"));
            let color = if model.is_broken() {
                theme::ERROR
            } else {
                theme::TEXT_SECONDARY
            };
            ui.label(egui::RichText::new(desc).size(12.0).color(color));
            egui::CollapsingHeader::new(i18n::t(lang, "technical_details"))
                .id_salt("tech-model")
                .default_open(false)
                .open(if self.expand_tech { Some(true) } else { None })
                .show(ui, |ui| {
                    let small = |ui: &mut egui::Ui, text: String| {
                        ui.label(egui::RichText::new(text).weak().size(11.5));
                    };
                    small(ui, i18n::tr(lang, "tech_model", &[&model.name]));
                    small(ui, i18n::tr(lang, "tech_backend", &[model.kind().label()]));
                    small(
                        ui,
                        i18n::tr(lang, "tech_scale", &[&model.scale.to_string()]),
                    );
                    small(ui, i18n::tr(lang, "tech_license", &[&model.license]));
                    if let Some(reason) = &model.broken_reason {
                        small(ui, i18n::tr(lang, "broken_reason", &[reason]));
                    }
                    if !model.notes.is_empty() {
                        small(ui, model.notes.clone());
                    }
                    if !model.source.is_empty() {
                        small(ui, model.source.clone());
                    }
                });
        }

        ui.add_space(16.0);
        ui.label(theme::subtitle_text(self.t("output_size")));
        ui.add_space(8.0);
        ui.add_enabled_ui(!self.running, |ui| {
            theme::segmented_frame().show(ui, |ui| {
                ui.spacing_mut().item_spacing.x = 0.0;
                ui.horizontal(|ui| {
                    if ui
                        .add(theme::segmented_button(
                            self.t("original_sharpen"),
                            self.keep_size,
                        ))
                        .on_hover_text(self.t("original_sharpen_tip"))
                        .clicked()
                    {
                        self.keep_size = true;
                    }
                    for value in [2, 3, 4] {
                        if ui
                            .add(theme::segmented_button(
                                format!("{value}×"),
                                !self.keep_size && self.scale == value,
                            ))
                            .clicked()
                        {
                            self.keep_size = false;
                            self.scale = value;
                        }
                    }
                });
            });
        });
        let model_scale = self.selected().map(|m| m.scale).unwrap_or(4);
        let hint = if self.keep_size {
            self.t("hint_keep_size").to_string()
        } else if self.scale == model_scale {
            self.tr("hint_native", &[&self.scale.to_string()])
        } else {
            self.tr("hint_rescale", &[&self.scale.to_string()])
        };
        ui.label(theme::caption_text(hint));

        ui.add_space(16.0);
        ui.horizontal(|ui| {
            ui.label(self.t("tile"));
            ui.add_enabled(
                !self.running,
                egui::DragValue::new(&mut self.tile)
                    .range(0..=1024)
                    .speed(16),
            );
            ui.label(theme::caption_text(self.t("tile_auto")));
        });
        ui.horizontal(|ui| {
            ui.label(self.t("overlap"));
            ui.add_enabled(
                !self.running,
                egui::DragValue::new(&mut self.overlap)
                    .range(0..=256)
                    .speed(4),
            );
            ui.label(theme::caption_text(self.t("onnx_seams")));
        });
        ui.label(theme::caption_text(self.t("tile_hint")));

        ui.add_space(16.0);
        ui.label(theme::subtitle_text(self.t("gpu")));
        ui.add_space(8.0);
        let gpu_label = self.gpu_label();
        egui::ComboBox::from_id_salt("gpu")
            .selected_text(gpu_label)
            .width(ui.available_width())
            .show_ui(ui, |ui| {
                let auto_gpu = self.t("auto");
                ui.selectable_value(&mut self.gpu, "auto".into(), auto_gpu);
                for device in &self.gpus {
                    ui.selectable_value(&mut self.gpu, device.id.clone(), &device.label);
                }
                ui.label(self.t("cpu_hint"));
            });

        ui.add_space(6.0);
        let tta_label = self.t("tta");
        let denoise_label = self.t("denoise_first");
        let face_restore_label = self.t("face_restore");
        let fast_auto_label = self.t("fast_auto");
        ui.add_enabled(
            !self.running && !onnx && (self.model_idx != self.models.len() || self.fast),
            egui::Checkbox::new(&mut self.tta, tta_label),
        );
        if onnx {
            ui.label(theme::caption_text(self.t("tta_onnx_hint")));
        }
        let has_denoise = self.models.iter().any(|m| m.denoise);
        ui.add_enabled(
            !self.running && has_denoise,
            egui::Checkbox::new(&mut self.denoise, denoise_label),
        );
        ui.add_enabled(
            !self.running,
            egui::Checkbox::new(&mut self.face_restore, face_restore_label),
        );
        ui.add_enabled_ui(!self.running && self.face_restore, |ui| {
            let face_strength_label = self.t("face_strength");
            ui.horizontal(|ui| {
                ui.label(face_strength_label);
                ui.add(egui::Slider::new(&mut self.face_strength, 0.0..=1.0));
            });
        });
        if self.model_idx == self.models.len() {
            ui.add_enabled(
                !self.running,
                egui::Checkbox::new(&mut self.fast, fast_auto_label),
            );
        }

        ui.add_space(16.0);
        ui.label(theme::subtitle_text(self.t("save_as")));
        ui.add_space(8.0);
        ui.horizontal(|ui| {
            ui.add_enabled_ui(!self.running, |ui| {
                let response = ui.add(
                    egui::TextEdit::singleline(&mut self.output_path)
                        .hint_text("enhanced.png")
                        .desired_width((ui.available_width() - 90.0).max(80.0)),
                );
                theme::underline_focus(ui, &response);
            });
            if ui
                .add_enabled(!self.running, theme::standard_button(self.t("browse")))
                .clicked()
            {
                self.pick_output();
            }
        });
        if let Some((w, h)) = self.output_size {
            ui.label(theme::caption_text(
                self.tr("last_result", &[&w.to_string(), &h.to_string()]),
            ));
        }

        ui.add_space(12.0);
        if self.running {
            if let Some(control) = &self.batch_control {
                let paused = control.paused.load(Ordering::SeqCst);
                if ui
                    .add(theme::standard_button(if paused {
                        self.t("resume")
                    } else {
                        self.t("pause")
                    }))
                    .clicked()
                {
                    control.paused.store(!paused, Ordering::SeqCst);
                }
            }
            if ui
                .add_sized(
                    [ui.available_width(), 32.0],
                    theme::danger_button(self.t("cancel")),
                )
                .clicked()
            {
                self.cancel_job();
            }
        } else {
            let ready = !self.input_path.trim().is_empty()
                && (self.model_idx == self.models.len()
                    || self.selected().is_some_and(|model| !model.is_broken()));
            let mut enhance = false;
            ui.add_enabled_ui(ready, |ui| {
                if ui
                    .add_sized(
                        [ui.available_width(), 32.0],
                        theme::accent_button(self.t("enhance")),
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
        if self.scroll_controls {
            ui.scroll_to_cursor_animation(
                Some(egui::Align::BOTTOM),
                egui::style::ScrollAnimation::none(),
            );
        }
    }

    fn preview(&mut self, ui: &mut egui::Ui) {
        let available = ui.available_size();
        let (rect, response) = ui.allocate_exact_size(available, egui::Sense::click_and_drag());
        let before_size = self.before.as_ref().map(|tex| tex.size_vec2());
        let before_id = self.before.as_ref().map(|tex| tex.id());
        let after_id = self.after.as_ref().map(|tex| tex.id());

        let painter = ui.painter_at(rect);
        painter.rect_filled(rect, 8.0, theme::WHITE);

        let Some(before_size) = before_size else {
            let dash = rect.shrink(20.0);
            theme::dashed_rounded_rect(
                &painter,
                dash,
                8.0,
                egui::Stroke::new(
                    1.5,
                    if self.hovering_file {
                        theme::ACCENT
                    } else {
                        theme::BORDER_DARK
                    },
                ),
            );
            theme::paint_image_glyph(
                &painter,
                dash.center() - egui::vec2(0.0, 28.0),
                48.0,
                if self.hovering_file {
                    theme::ACCENT
                } else {
                    theme::TEXT_SECONDARY
                },
            );
            painter.text(
                dash.center() + egui::vec2(0.0, 28.0),
                egui::Align2::CENTER_CENTER,
                self.t("drop_here"),
                egui::FontId::proportional(14.0),
                theme::TEXT_SECONDARY,
            );
            return;
        };
        let Some(before_id) = before_id else { return };

        let aspect = if before_size.y <= 0.0 {
            1.0
        } else {
            before_size.x / before_size.y
        };
        let fitted = fit_rect(rect.shrink(16.0), aspect);

        if response.double_clicked() {
            self.zoom = 1.0;
            self.pan = egui::Vec2::ZERO;
        }
        if response.hovered() {
            let (scroll, pointer) = ui.input(|i| (i.smooth_scroll_delta.y, i.pointer.hover_pos()));
            if scroll != 0.0 {
                let old = self.zoom;
                self.zoom = (self.zoom * (1.0 + scroll * 0.001)).clamp(0.25, 8.0);
                if let Some(pos) = pointer {
                    self.pan += (pos - fitted.center()) * (1.0 - self.zoom / old);
                }
            }
        }
        // At fit size or smaller the image stays pinned to the center. When
        // zoomed in, keep the pan inside the image so it never drifts away.
        if self.zoom <= 1.0 {
            self.pan = egui::Vec2::ZERO;
        } else {
            let max = (fitted.size() * self.zoom - fitted.size()) * 0.5;
            self.pan = egui::vec2(
                self.pan.x.clamp(-max.x, max.x),
                self.pan.y.clamp(-max.y, max.y),
            );
        }
        let fitted =
            egui::Rect::from_center_size(fitted.center() + self.pan, fitted.size() * self.zoom);

        // The split line lives in the visible part of the image, so it stays on
        // screen and draggable at any zoom level.
        let visible = fitted.intersect(rect);
        let split_x = |split: f32| visible.left() + visible.width() * split;
        if response.drag_started_by(egui::PointerButton::Primary) {
            let near_line = response
                .interact_pointer_pos()
                .is_some_and(|pos| (pos.x - split_x(self.split)).abs() <= 14.0);
            self.dragging_split = near_line || self.zoom <= 1.01;
        }
        if response.drag_stopped() {
            self.dragging_split = false;
        }
        if response.dragged_by(egui::PointerButton::Middle)
            || (response.dragged_by(egui::PointerButton::Primary) && !self.dragging_split)
        {
            if self.zoom > 1.0 {
                self.pan += response.drag_delta();
            }
        } else if response.dragged_by(egui::PointerButton::Primary) && self.dragging_split {
            if let Some(pos) = response.interact_pointer_pos() {
                if visible.width() > 1.0 {
                    self.split = ((pos.x - visible.left()) / visible.width()).clamp(0.02, 0.98);
                }
            }
        }
        if let Some(pos) = response.hover_pos() {
            if self.dragging_split || (pos.x - split_x(self.split)).abs() <= 14.0 {
                ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeHorizontal);
            } else if self.zoom > 1.01 {
                ui.ctx().set_cursor_icon(egui::CursorIcon::Grab);
            }
        }

        let uv = egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0));
        if let Some(after_id) = after_id {
            painter.image(after_id, fitted, uv, egui::Color32::WHITE);
            let clip = egui::Rect::from_min_max(
                fitted.min,
                egui::pos2(split_x(self.split), fitted.bottom()),
            );
            painter
                .with_clip_rect(clip)
                .image(before_id, fitted, uv, egui::Color32::WHITE);
            let x = split_x(self.split);
            painter.line_segment(
                [
                    egui::pos2(x, visible.top()),
                    egui::pos2(x, visible.bottom()),
                ],
                egui::Stroke::new(2.0, egui::Color32::WHITE),
            );
            painter.circle_filled(egui::pos2(x, visible.center().y), 7.0, egui::Color32::WHITE);
            theme::pill(
                &painter,
                egui::pos2(fitted.left() + 8.0, fitted.top() + 8.0),
                self.t("before"),
                false,
            );
            theme::pill(
                &painter,
                egui::pos2(fitted.right() - 8.0, fitted.top() + 8.0),
                self.t("after"),
                true,
            );
        } else {
            painter.image(before_id, fitted, uv, egui::Color32::WHITE);
            theme::pill(
                &painter,
                egui::pos2(fitted.left() + 8.0, fitted.top() + 8.0),
                self.t("original"),
                false,
            );
        }

        if self.hovering_file {
            theme::dashed_rounded_rect(
                &painter,
                rect.shrink(6.0),
                8.0,
                egui::Stroke::new(2.0, theme::ACCENT),
            );
        }
    }

    fn t(&self, key: &str) -> &'static str {
        i18n::t(self.lang, key)
    }

    fn tr(&self, key: &str, args: &[&str]) -> String {
        i18n::tr(self.lang, key, args)
    }

    fn set_lang(&mut self, lang: Lang) {
        if self.lang == lang {
            return;
        }
        self.status = i18n::localize_runtime(lang, &self.status);
        self.error = i18n::localize_runtime(lang, &self.error);
        self.lang = lang;
        if self.persist_lang {
            i18n::save_lang(lang);
        }
    }

    fn gpu_label(&self) -> String {
        if self.gpu == "auto" {
            return self.t("auto").to_string();
        }
        if self.gpu == "cpu" {
            return self.t("cpu").to_string();
        }
        self.gpus
            .iter()
            .find(|g| g.id == self.gpu)
            .map(|g| g.label.clone())
            .unwrap_or_else(|| self.tr("gpu_n", &[&self.gpu]))
    }

    fn current_elapsed(&self) -> Option<Duration> {
        if self.running {
            self.started.map(|t| t.elapsed())
        } else {
            self.elapsed
        }
    }
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
