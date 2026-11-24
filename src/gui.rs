use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use eframe::egui;

use crate::batch::BatchEvent;
use crate::engine::{self, EngineEvent, GpuChoice, GpuDevice, JobOptions, RunOutcome};
use crate::i18n::{self, Lang};
use crate::icons::{self, Icon};
use crate::models::{self, ModelInfo};
use crate::paths;
use crate::theme;
use crate::thumbs::ThumbCache;

pub struct ScreenshotSpec {
    pub output: PathBuf,
    pub input: Option<PathBuf>,
    pub after: Option<PathBuf>,
    pub lang: Option<Lang>,
    pub expand: bool,
    pub scroll: bool,
    pub queue: Option<PathBuf>,
    pub fake_queue_states: bool,
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
        [1520.0, 980.0]
    } else {
        [1360.0, 900.0]
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
    BatchEvent(BatchEvent),
}

#[derive(Clone)]
enum QueueStatus {
    Pending,
    Processing,
    Done {
        output: PathBuf,
    },
    Failed {
        #[allow(dead_code)]
        error: String,
    },
}

struct QueueItem {
    path: PathBuf,
    status: QueueStatus,
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
    queue: Vec<QueueItem>,
    queue_selected: usize,
    thumbs: ThumbCache,
    #[allow(dead_code)]
    card_open: [bool; 4],
    #[allow(dead_code)]
    compare_on: bool,
    show_result: bool,
    #[allow(dead_code)]
    pan_tool: bool,
    #[allow(dead_code)]
    about_open: bool,
    #[allow(dead_code)]
    show_log: bool,
    #[allow(dead_code)]
    more_open: bool,
    batch_jobs: usize,
    batch_queue: bool,
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
            queue: Vec::new(),
            queue_selected: 0,
            thumbs: ThumbCache::new(),
            card_open: [true; 4],
            compare_on: true,
            show_result: false,
            pan_tool: false,
            about_open: false,
            show_log: false,
            more_open: false,
            batch_jobs: 1,
            batch_queue: false,
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
        if let Some(path) = spec.input.clone() {
            self.load_input(ctx, path);
        }
        if let Some(path) = spec.after.clone() {
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
        if let Some(dir) = &spec.queue {
            if let Ok(files) = crate::batch::scan_folder(dir) {
                self.replace_queue(files);
                if let Some(input) = spec.input.as_ref() {
                    if let Some(index) = self.queue.iter().position(|item| &item.path == input) {
                        self.queue_selected = index;
                    }
                }
                if spec.fake_queue_states {
                    let after = spec.after.clone();
                    for (i, item) in self.queue.iter_mut().enumerate() {
                        item.status = match i {
                            0 => QueueStatus::Done {
                                output: after.clone().unwrap_or_else(|| item.path.clone()),
                            },
                            1 => QueueStatus::Processing,
                            2 => QueueStatus::Failed {
                                error: "screenshot".into(),
                            },
                            _ => QueueStatus::Pending,
                        };
                    }
                }
                if let Some(first) = self.queue.first().map(|item| item.path.clone()) {
                    if spec.input.is_none() {
                        self.load_input(ctx, first);
                    }
                    if let Some(QueueStatus::Done { output }) =
                        self.queue.first().map(|item| item.status.clone())
                    {
                        if self.after.is_none() {
                            if let Ok((rgba, w, h)) = engine::make_preview(&output, 1600) {
                                self.after = Some(load_texture(ctx, "after", &rgba, w, h));
                            }
                        }
                    }
                }
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
        let thumbs_ready =
            self.queue.is_empty() || self.thumbs.ready_count() >= self.queue.len().min(8);
        if (job.frames >= 28 || thumbs_ready && job.frames >= 10) && !job.requested {
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
        if self.rx.is_none() {
            return;
        }
        loop {
            let message = match self.rx.as_ref().unwrap().try_recv() {
                Ok(msg) => msg,
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => {
                    if self.running {
                        self.running = false;
                        self.status = self.t("failed").to_string();
                        self.error = self.t("thread_exited").to_string();
                    }
                    break;
                }
            };
            match message {
                WorkerMsg::Event(EngineEvent::Percent(p)) => {
                    self.progress = p;
                    self.status = self.tr("upscaling_pct", &[&format!("{p:.1}")]);
                }
                WorkerMsg::Event(EngineEvent::Stage(text)) => {
                    self.status = i18n::plain_stage(self.lang, &text);
                    push_log(&mut self.log, &text);
                }
                WorkerMsg::Event(EngineEvent::Log(line)) => {
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
                WorkerMsg::Finished(Ok(outcome)) => {
                    self.running = false;
                    self.elapsed = Some(outcome.elapsed);
                    self.progress = 100.0;
                    self.output_size = Some(outcome.output_size);
                    self.mark_queue_done(&outcome.output);
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
                WorkerMsg::Finished(Err(message)) => {
                    self.mark_queue_failed(&message);
                    self.running = false;
                    if let Some(start) = self.started {
                        self.elapsed = Some(start.elapsed());
                    }
                    self.status = self.t("failed").to_string();
                    self.error = i18n::localize_runtime(self.lang, &message);
                }
                WorkerMsg::BatchFinished(result) => {
                    self.running = false;
                    self.batch_control = None;
                    self.status = match result {
                        Ok(()) => self.t("batch_complete").to_string(),
                        Err(e) => self.tr("batch_stopped", &[&e]),
                    };
                }
                WorkerMsg::BatchEvent(event) => self.apply_batch_event(event),
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
            self.batch_queue = false;
            self.push_queue_path(path.clone());
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
        if let Some(item) = self.queue.get_mut(self.queue_selected) {
            item.status = QueueStatus::Processing;
        }

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
        let Some(first) = self.queue.first().map(|item| item.path.clone()) else {
            return;
        };
        let input_dir = first.parent().unwrap_or(Path::new(".")).to_path_buf();
        let files = self.queue_paths();
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
            let result = crate::batch::run(&root, &opts, control, move |ev| match ev {
                BatchEvent::Line(line) => {
                    let _ = tx_report.send(WorkerMsg::Event(EngineEvent::Log(line)));
                }
                other => {
                    let _ = tx_report.send(WorkerMsg::BatchEvent(other));
                }
            });
            let _ = tx.send(WorkerMsg::BatchFinished(
                result.map_err(|e| format!("{e:#}")),
            ));
        });
    }

    fn queue_paths(&self) -> Vec<PathBuf> {
        self.queue.iter().map(|item| item.path.clone()).collect()
    }

    fn replace_queue(&mut self, files: Vec<PathBuf>) {
        self.batch_queue = true;
        self.queue = files
            .into_iter()
            .map(|path| QueueItem {
                path,
                status: QueueStatus::Pending,
            })
            .collect();
        self.queue_selected = 0;
        let keep = self.queue_paths();
        self.thumbs.retain(&keep);
    }

    fn extend_queue(&mut self, files: Vec<PathBuf>) {
        self.batch_queue = true;
        for path in files {
            self.push_queue_path(path);
        }
        let keep = self.queue_paths();
        self.thumbs.retain(&keep);
    }

    fn push_queue_path(&mut self, path: PathBuf) {
        if let Some(idx) = self.queue.iter().position(|item| item.path == path) {
            self.queue_selected = idx;
            return;
        }
        self.queue.push(QueueItem {
            path,
            status: QueueStatus::Pending,
        });
        self.queue_selected = self.queue.len().saturating_sub(1);
    }

    fn apply_batch_event(&mut self, event: BatchEvent) {
        match event {
            BatchEvent::Line(_) => {}
            BatchEvent::FileStarted { path, .. } => {
                if let Some(item) = self.queue.iter_mut().find(|item| item.path == path) {
                    item.status = QueueStatus::Processing;
                }
            }
            BatchEvent::FileFinished { path, output, .. }
            | BatchEvent::FileSkipped { path, output, .. } => {
                if let Some(item) = self.queue.iter_mut().find(|item| item.path == path) {
                    item.status = QueueStatus::Done { output };
                }
            }
            BatchEvent::FileFailed { path, error, .. } => {
                if let Some(item) = self.queue.iter_mut().find(|item| item.path == path) {
                    item.status = QueueStatus::Failed { error };
                }
            }
        }
    }

    fn mark_queue_done(&mut self, output: &Path) {
        let input = PathBuf::from(self.input_path.trim());
        let idx = self
            .queue
            .iter()
            .position(|item| item.path == input)
            .unwrap_or(self.queue_selected);
        if let Some(item) = self.queue.get_mut(idx) {
            item.status = QueueStatus::Done {
                output: output.to_path_buf(),
            };
        }
    }

    fn mark_queue_failed(&mut self, message: &str) {
        if let Some(item) = self.queue.get_mut(self.queue_selected) {
            item.status = QueueStatus::Failed {
                error: message.to_string(),
            };
        }
    }

    fn select_queue_item(&mut self, ctx: &egui::Context, index: usize) {
        let Some(item) = self.queue.get(index) else {
            return;
        };
        let path = item.path.clone();
        let output = match &item.status {
            QueueStatus::Done { output } => Some(output.clone()),
            _ => None,
        };
        self.queue_selected = index;
        self.load_input(ctx, path);
        if let Some(output) = output {
            if let Ok((rgba, w, h)) = engine::make_preview(&output, 1600) {
                self.after = Some(load_texture(ctx, "after", &rgba, w, h));
                if let Ok(size) = image::image_dimensions(&output) {
                    self.output_size = Some(size);
                }
                self.output_path = output.display().to_string();
                self.progress = 100.0;
            }
        }
    }
}

impl eframe::App for InfScalerApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        self.poll(&ctx);
        self.thumbs.poll(&ctx);
        let queued_paths = self.queue_paths();
        self.thumbs.retain(&queued_paths);
        for path in &queued_paths {
            self.thumbs.request(path);
        }
        self.hovering_file = ctx.input(|i| !i.raw.hovered_files.is_empty());
        let dropped: Vec<PathBuf> = ctx.input(|i| {
            i.raw
                .dropped_files
                .iter()
                .filter_map(|file| {
                    let path = file.path();
                    if path.is_dir() || engine::is_image_path(path) {
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
                        let was_empty = self.queue.is_empty();
                        self.extend_queue(files);
                        if was_empty && !self.queue.is_empty() {
                            self.select_queue_item(&ctx, 0);
                        }
                    }
                } else {
                    self.batch_queue = false;
                    self.push_queue_path(path.clone());
                    self.load_input(&ctx, path);
                }
            }
        }
        if ctx.input(|i| i.modifiers.command && i.key_pressed(egui::Key::O)) && !self.running {
            self.pick_input(&ctx);
        }

        if self.running || !self.error.is_empty() || self.show_log {
            egui::Panel::bottom("status")
                .frame(theme::slim_status_panel())
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        if self.running {
                            ui.add(
                                egui::ProgressBar::new((self.progress / 100.0).clamp(0.0, 1.0))
                                    .fill(theme::ACCENT)
                                    .desired_width((ui.available_width() * 0.55).max(100.0)),
                            );
                        }
                        ui.label(
                            egui::RichText::new(&self.status)
                                .color(theme::TEXT)
                                .size(13.0),
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
                        ui.colored_label(theme::ERROR, egui::RichText::new(&self.error).size(12.0));
                    }
                    if self.show_log && !self.log.is_empty() {
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
        }

        egui::Panel::left("controls")
            .resizable(true)
            .default_size(400.0)
            .min_size(360.0)
            .max_size(440.0)
            .show_separator_line(false)
            .frame(theme::mica_panel())
            .show(ui, |ui| {
                let height = ui.available_height();
                let button_h = if self.running { 76.0 } else { 52.0 };
                egui::ScrollArea::vertical()
                    .id_salt("controls-scroll")
                    .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysHidden)
                    .auto_shrink([false, false])
                    .max_height((height - button_h - 18.0).max(64.0))
                    .show(ui, |ui| {
                        ui.set_width(ui.available_width());
                        self.controls(ui, &ctx);
                    });
                ui.add_space(6.0);
                if self.running {
                    ui.horizontal(|ui| {
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
                        if ui.add(theme::danger_button(self.t("cancel"))).clicked() {
                            self.cancel_job();
                        }
                    });
                } else {
                    let ready = !self.input_path.trim().is_empty()
                        && (self.model_idx == self.models.len()
                            || self.selected().is_some_and(|m| !m.is_broken()));
                    let response = icons::labeled(
                        ui,
                        Icon::Play,
                        self.t("enhance"),
                        true,
                        ready,
                        ui.available_width(),
                    );
                    if response.clicked() {
                        if self.batch_queue {
                            self.start_batch();
                        } else {
                            self.start_job();
                        }
                    }
                }
            });

        egui::CentralPanel::default()
            .frame(theme::mica_panel())
            .show(ui, |ui| {
                let available = ui.available_size();
                let preview_h = (available.y - 190.0).max(240.0);
                ui.allocate_ui_with_layout(
                    egui::vec2(available.x, preview_h),
                    egui::Layout::top_down(egui::Align::Min),
                    |ui| {
                        theme::card_frame().show(ui, |ui| {
                            self.preview(ui);
                        });
                        ui.add_space(10.0);
                    },
                );
                ui.allocate_ui_with_layout(
                    egui::vec2(available.x, (available.y - preview_h - 10.0).max(150.0)),
                    egui::Layout::top_down(egui::Align::Min),
                    |ui| {
                        self.queue_strip(ui, &ctx);
                    },
                );
            });
        self.tick_screenshot(&ctx);
    }
}

impl InfScalerApp {
    fn controls(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        ui.spacing_mut().item_spacing.y = 2.0;
        ui.add_space(2.0);
        let input_title = self.t("input_photo");
        let mut input_open = self.card_open[0];
        theme::collapsing_card(
            ui,
            "input-card",
            Icon::Image,
            input_title,
            &mut input_open,
            |ui| {
                ui.horizontal(|ui| {
                    let open = icons::labeled(
                        ui,
                        Icon::Open,
                        self.t("open_photo_btn"),
                        true,
                        !self.running,
                        116.0,
                    );
                    if open.clicked() {
                        self.pick_input(ctx);
                    }
                    let drop = icons::dashed_label_button(
                        ui,
                        Icon::Drop,
                        self.t("drop_on_window"),
                        self.hovering_file,
                    );
                    drop.on_hover_text(self.t("drop_here"));
                });
                ui.add_space(3.0);
                ui.horizontal(|ui| {
                    ui.label(self.t("output_folder_label"));
                    let label = self
                        .batch_output_dir
                        .as_ref()
                        .map(|p| p.display().to_string())
                        .unwrap_or_else(|| self.t("auto").to_string());
                    egui::ComboBox::from_id_salt("output-folder")
                        .selected_text(label)
                        .width((ui.available_width() - 38.0).max(90.0))
                        .show_ui(ui, |ui| {
                            if ui
                                .selectable_label(self.batch_output_dir.is_none(), self.t("auto"))
                                .clicked()
                            {
                                self.batch_output_dir = None;
                            }
                            if let Some(path) = &self.batch_output_dir {
                                let path = path.clone();
                                if ui
                                    .selectable_label(true, path.display().to_string())
                                    .clicked()
                                {
                                    self.batch_output_dir = Some(path);
                                }
                            }
                        });
                    if icons::icon_button(ui, Icon::Folder, false, self.t("choose_output_folder"))
                        .clicked()
                        && !self.running
                    {
                        if let Some(path) = rfd::FileDialog::new()
                            .set_title(self.t("choose_output_folder"))
                            .pick_folder()
                        {
                            self.batch_output_dir = Some(path);
                        }
                    }
                });
                ui.horizontal(|ui| {
                    ui.label(self.t("onnx_threads"))
                        .on_hover_text(self.t("threads_hint"));
                    ui.add_enabled(
                        !self.running,
                        egui::DragValue::new(&mut self.batch_threads).range(1..=12),
                    );
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui
                            .add_enabled(
                                !self.running,
                                theme::standard_button(self.t("open_folder")),
                            )
                            .clicked()
                        {
                            if let Some(folder) = rfd::FileDialog::new()
                                .set_title(self.t("open_image_folder"))
                                .pick_folder()
                            {
                                if let Ok(files) = crate::batch::scan_folder(&folder) {
                                    let n = files.len();
                                    self.replace_queue(files);
                                    if !self.queue.is_empty() {
                                        self.select_queue_item(&ctx, 0);
                                    }
                                    self.status = self.tr("queued_top_level", &[&n.to_string()]);
                                }
                            }
                        }
                    });
                });
                ui.label(theme::caption_text(
                    self.tr("queue_count", &[&self.queue.len().to_string()]),
                ));
                egui::ComboBox::from_id_salt("batch-preset")
                    .selected_text(i18n::preset_label(self.lang, &self.batch_preset))
                    .width(ui.available_width())
                    .show_ui(ui, |ui| {
                        for p in ["general", "people", "fast"] {
                            ui.selectable_value(
                                &mut self.batch_preset,
                                p.into(),
                                i18n::preset_label(self.lang, p),
                            );
                        }
                    });
                ui.add_enabled_ui(!self.running, |ui| {
                    theme::segmented_frame().show(ui, |ui| {
                        ui.spacing_mut().item_spacing.x = 0.0;
                        ui.horizontal(|ui| {
                            let w = ui.available_width() / 3.0;
                            for n in 1..=3 {
                                if ui
                                    .add_sized(
                                        [w, 28.0],
                                        theme::segmented_button(
                                            i18n::jobs_label(self.lang, n),
                                            self.batch_jobs == n,
                                        ),
                                    )
                                    .clicked()
                                {
                                    self.batch_jobs = n;
                                }
                            }
                        });
                    });
                    let response = ui.add(
                        egui::TextEdit::singleline(&mut self.input_path)
                            .hint_text("input.jpg")
                            .desired_width(f32::INFINITY),
                    );
                    theme::underline_focus(ui, &response);
                });
            },
        );
        self.card_open[0] = input_open;
        ui.add_space(3.0);

        let style_title = self.t("model");
        let mut style_open = self.card_open[1];
        theme::collapsing_card(
            ui,
            "style-card",
            Icon::Sparkle,
            style_title,
            &mut style_open,
            |ui| {
                let selected = if self.model_idx == self.models.len() {
                    self.t("auto_choose").to_string()
                } else {
                    self.selected()
                        .map(|m| m.display_name(self.lang))
                        .unwrap_or_else(|| self.t("none").to_string())
                };
                let auto_choose = self.t("auto_choose");
                egui::ComboBox::from_id_salt("model")
                    .selected_text(selected)
                    .width(ui.available_width())
                    .show_ui(ui, |ui| {
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
                    ui.label(egui::RichText::new(self.t("desc_auto")).size(12.0));
                    egui::CollapsingHeader::new(self.t("technical_details"))
                        .id_salt("tech-auto")
                        .default_open(false)
                        .open(if self.expand_tech { Some(true) } else { None })
                        .show(ui, |ui| {
                            ui.label(
                                egui::RichText::new(i18n::t(lang, "tech_auto"))
                                    .weak()
                                    .size(11.0),
                            );
                        });
                } else if let Some(model) = self.selected() {
                    let desc = i18n::model_description(lang, &model.name)
                        .unwrap_or_else(|| i18n::t(lang, "desc_custom"));
                    ui.label(
                        egui::RichText::new(desc)
                            .size(12.0)
                            .color(if model.is_broken() {
                                theme::ERROR
                            } else {
                                theme::TEXT_SECONDARY
                            }),
                    );
                    egui::CollapsingHeader::new(i18n::t(lang, "technical_details"))
                        .id_salt("tech-model")
                        .default_open(false)
                        .open(if self.expand_tech { Some(true) } else { None })
                        .show(ui, |ui| {
                            ui.label(
                                egui::RichText::new(i18n::tr(lang, "tech_model", &[&model.name]))
                                    .weak()
                                    .size(11.0),
                            );
                            ui.label(
                                egui::RichText::new(i18n::tr(
                                    lang,
                                    "tech_backend",
                                    &[model.kind().label()],
                                ))
                                .weak()
                                .size(11.0),
                            );
                            ui.label(
                                egui::RichText::new(i18n::tr(
                                    lang,
                                    "tech_scale",
                                    &[&model.scale.to_string()],
                                ))
                                .weak()
                                .size(11.0),
                            );
                            ui.label(
                                egui::RichText::new(i18n::tr(
                                    lang,
                                    "tech_license",
                                    &[&model.license],
                                ))
                                .weak()
                                .size(11.0),
                            );
                            if let Some(reason) = &model.broken_reason {
                                ui.label(
                                    egui::RichText::new(i18n::tr(lang, "broken_reason", &[reason]))
                                        .weak()
                                        .size(11.0),
                                );
                            }
                            if !model.notes.is_empty() {
                                ui.label(
                                    egui::RichText::new(model.notes.clone()).weak().size(11.0),
                                );
                            }
                            if !model.source.is_empty() {
                                ui.label(
                                    egui::RichText::new(model.source.clone()).weak().size(11.0),
                                );
                            }
                        });
                }
            },
        );
        self.card_open[1] = style_open;
        ui.add_space(3.0);

        let size_title = self.t("output_size");
        let mut size_open = self.card_open[2];
        theme::collapsing_card(
            ui,
            "size-card",
            Icon::Resize,
            size_title,
            &mut size_open,
            |ui| {
                ui.add_enabled_ui(!self.running, |ui| {
                    theme::segmented_frame().show(ui, |ui| {
                        ui.spacing_mut().item_spacing.x = 0.0;
                        ui.horizontal(|ui| {
                            let w = ui.available_width() / 4.0;
                            let options = [
                                (self.t("original_short").to_string(), 1),
                                ("2×".into(), 2),
                                ("3×".into(), 3),
                                ("4×".into(), 4),
                            ];
                            for (label, value) in options {
                                if ui
                                    .add_sized(
                                        [w, 28.0],
                                        theme::segmented_button(
                                            label,
                                            if value == 1 {
                                                self.keep_size
                                            } else {
                                                !self.keep_size && self.scale == value
                                            },
                                        ),
                                    )
                                    .clicked()
                                {
                                    self.keep_size = value == 1;
                                    if value > 1 {
                                        self.scale = value;
                                    }
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
                ui.horizontal(|ui| {
                    ui.label(self.t("tile"));
                    theme::stepper(ui, &mut self.tile, 0..=1024);
                    ui.label(theme::caption_text(self.t("tile_auto")));
                });
                ui.horizontal(|ui| {
                    ui.label(self.t("overlap"));
                    theme::stepper(ui, &mut self.overlap, 0..=256);
                    ui.label(theme::caption_text(self.t("onnx_seams")));
                });
                ui.label(theme::caption_text(self.t("tile_hint")));
            },
        );
        self.card_open[2] = size_open;
        ui.add_space(3.0);

        let gpu_title = self.t("gpu");
        let mut gpu_open = self.card_open[3];
        theme::collapsing_card(ui, "gpu-card", Icon::Gpu, gpu_title, &mut gpu_open, |ui| {
            ui.spacing_mut().interact_size.y = 22.0;
            ui.spacing_mut().item_spacing.y = 1.0;
            let auto_label = self.t("auto");
            let tta_label = self.t("tta");
            let denoise_label = self.t("denoise_first");
            let face_label = self.t("face_restore");
            let fast_label = self.t("fast_auto");
            egui::ComboBox::from_id_salt("gpu")
                .selected_text(self.gpu_label())
                .width(ui.available_width())
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut self.gpu, "auto".into(), auto_label);
                    for device in &self.gpus {
                        ui.selectable_value(&mut self.gpu, device.id.clone(), &device.label);
                    }
                    ui.label(self.t("cpu_hint"));
                });
            let onnx = self.selected().is_some_and(|m| m.is_onnx());
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
                egui::Checkbox::new(&mut self.face_restore, face_label),
            );
            ui.add_enabled_ui(!self.running && self.face_restore, |ui| {
                ui.horizontal(|ui| {
                    ui.label(self.t("face_strength"));
                    ui.add(egui::Slider::new(&mut self.face_strength, 0.0..=1.0));
                });
            });
            if self.model_idx == self.models.len() {
                ui.add_enabled(
                    !self.running,
                    egui::Checkbox::new(&mut self.fast, fast_label),
                );
            }
        });
        self.card_open[3] = gpu_open;
        ui.add_space(4.0);
        if self.scroll_controls {
            ui.scroll_to_cursor_animation(
                Some(egui::Align::BOTTOM),
                egui::style::ScrollAnimation::none(),
            );
        }
    }

    fn preview(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            icons::paint(
                ui,
                Icon::Image,
                ui.next_widget_position() + egui::vec2(9.0, 10.0),
                18.0,
                theme::TEXT,
            );
            ui.add_space(24.0);
            ui.label(theme::section_text(self.t("preview")));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let more = icons::icon_button(ui, Icon::More, false, self.t("more_menu"));
                more.context_menu(|ui| {
                    if ui.button(self.t("open_output_folder")).clicked() {
                        let target = self
                            .batch_output_dir
                            .clone()
                            .or_else(|| {
                                Path::new(&self.output_path).parent().map(Path::to_path_buf)
                            })
                            .or_else(|| std::env::current_dir().ok());
                        if let Some(path) = target {
                            let _ = std::process::Command::new("xdg-open").arg(path).spawn();
                        }
                        ui.close();
                    }
                    if ui.button(self.t("save_as")).clicked() {
                        self.pick_output();
                        ui.close();
                    }
                    if ui.button(self.t("about")).clicked() {
                        self.about_open = true;
                        ui.close();
                    }
                    if ui
                        .button(if self.show_log {
                            self.t("hide_log")
                        } else {
                            self.t("show_log")
                        })
                        .clicked()
                    {
                        self.show_log = !self.show_log;
                        ui.close();
                    }
                });
                if icons::labeled(
                    ui,
                    Icon::Refresh,
                    self.t("reload_models"),
                    false,
                    !self.running,
                    0.0,
                )
                .clicked()
                    && !self.running
                {
                    self.reload_models();
                    self.status = self.tr("model_count", &[&self.models.len().to_string()]);
                }
                ui.allocate_ui_with_layout(
                    egui::vec2(210.0, 32.0),
                    egui::Layout::left_to_right(egui::Align::Center),
                    |ui| {
                        ui.horizontal(|ui| {
                            let (globe_rect, _) = ui
                                .allocate_exact_size(egui::vec2(20.0, 28.0), egui::Sense::hover());
                            icons::paint(
                                ui,
                                Icon::Globe,
                                globe_rect.center(),
                                16.0,
                                theme::TEXT_SECONDARY,
                            );
                            let current = self.lang;
                            for lang in Lang::all().into_iter().rev() {
                                if ui
                                    .add(theme::segmented_button(lang.label(), current == lang))
                                    .clicked()
                                    && current != lang
                                {
                                    self.set_lang(lang);
                                }
                            }
                        });
                    },
                );
            });
        });
        ui.add_space(8.0);
        if self.about_open {
            let about_blurb = self.t("about_blurb");
            egui::Window::new(self.t("about"))
                .open(&mut self.about_open)
                .collapsible(false)
                .resizable(false)
                .show(ui.ctx(), |ui| {
                    ui.label("InfScaler");
                    ui.label(about_blurb);
                });
        }
        let has_after = self.after.is_some();
        ui.horizontal(|ui| {
            ui.add_space(4.0);
            if ui
                .add(theme::segmented_button(
                    self.t("original_short"),
                    !self.show_result,
                ))
                .clicked()
            {
                self.show_result = false;
                self.compare_on = false;
            }
            if ui
                .add_enabled(
                    has_after,
                    theme::segmented_button(self.t("result"), self.show_result),
                )
                .clicked()
            {
                self.show_result = true;
                self.compare_on = false;
            }
        });
        let image_h = (ui.available_height() - 44.0).max(80.0);
        let (rect, response) = ui.allocate_exact_size(
            egui::vec2(ui.available_width(), image_h),
            egui::Sense::click_and_drag(),
        );
        let painter = ui.painter_at(rect);
        painter.rect_filled(rect, 6.0, theme::WHITE);
        let Some(before) = self.before.as_ref() else {
            let dash = rect.shrink(18.0);
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
            icons::paint(
                ui,
                Icon::Image,
                dash.center() - egui::vec2(0.0, 20.0),
                48.0,
                theme::TEXT_SECONDARY,
            );
            painter.text(
                dash.center() + egui::vec2(0.0, 26.0),
                egui::Align2::CENTER_CENTER,
                self.t("drop_here"),
                egui::FontId::proportional(14.0),
                theme::TEXT_SECONDARY,
            );
            return;
        };
        let before_size = before.size_vec2();
        let before_id = before.id();
        let after_id = self.after.as_ref().map(|t| t.id());
        let aspect = if before_size.y <= 0.0 {
            1.0
        } else {
            before_size.x / before_size.y
        };
        let base = fit_rect(rect.shrink(14.0), aspect);
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
                    self.pan += (pos - base.center()) * (1.0 - self.zoom / old);
                }
            }
        }
        if self.zoom <= 1.0 {
            self.pan = egui::Vec2::ZERO;
        } else {
            let max = (base.size() * self.zoom - base.size()) * 0.5;
            self.pan = egui::vec2(
                self.pan.x.clamp(-max.x, max.x),
                self.pan.y.clamp(-max.y, max.y),
            );
        }
        let fitted =
            egui::Rect::from_center_size(base.center() + self.pan, base.size() * self.zoom);
        let visible = fitted.intersect(rect);
        let split_x = |split: f32| visible.left() + visible.width() * split;
        if response.drag_started_by(egui::PointerButton::Primary) {
            self.dragging_split = self.compare_on
                && after_id.is_some()
                && response
                    .interact_pointer_pos()
                    .is_some_and(|p| (p.x - split_x(self.split)).abs() <= 14.0);
        }
        if response.drag_stopped() {
            self.dragging_split = false;
        }
        if response.dragged_by(egui::PointerButton::Middle)
            || (response.dragged_by(egui::PointerButton::Primary)
                && !self.dragging_split
                && self.pan_tool)
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
        let uv = egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0));
        if self.compare_on && after_id.is_some() {
            painter.image(after_id.unwrap(), fitted, uv, egui::Color32::WHITE);
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
        } else if self.show_result {
            if let Some(id) = after_id {
                painter.image(id, fitted, uv, egui::Color32::WHITE);
            } else {
                painter.image(before_id, fitted, uv, egui::Color32::WHITE);
            }
        } else {
            painter.image(before_id, fitted, uv, egui::Color32::WHITE);
        }
        if self.hovering_file {
            theme::dashed_rounded_rect(
                &painter,
                rect.shrink(6.0),
                8.0,
                egui::Stroke::new(2.0, theme::ACCENT),
            );
        }

        ui.horizontal(|ui| {
            ui.add_space((ui.available_width() * 0.28).max(0.0));
            if icons::icon_button(ui, Icon::Fit, false, self.t("fit_to_window")).clicked() {
                self.zoom = 1.0;
                self.pan = egui::Vec2::ZERO;
            }
            let native = self
                .input_size
                .unwrap_or((before_size.x as u32, before_size.y as u32));
            let fit_scale = if native.0 > 0 {
                base.width() / native.0 as f32
            } else {
                1.0
            };
            let pct = (self.zoom * fit_scale * 100.0).round() as i32;
            egui::ComboBox::from_id_salt("zoom-pct")
                .selected_text(format!("{pct}%"))
                .width(74.0)
                .show_ui(ui, |ui| {
                    for value in [25, 50, 100, 200, 400] {
                        if ui
                            .selectable_label(pct == value, format!("{value}%"))
                            .clicked()
                        {
                            self.zoom =
                                ((value as f32 / 100.0) / fit_scale.max(0.001)).clamp(0.25, 8.0);
                            self.pan = egui::Vec2::ZERO;
                        }
                    }
                    if ui
                        .selectable_label((self.zoom - 1.0).abs() < 0.01, self.t("zoom_fit"))
                        .clicked()
                    {
                        self.zoom = 1.0;
                        self.pan = egui::Vec2::ZERO;
                    }
                });
            if icons::icon_button(ui, Icon::Minus, false, self.t("zoom")).clicked() {
                self.zoom = (self.zoom / 1.25).clamp(0.25, 8.0);
            }
            if icons::icon_button(ui, Icon::Plus, false, self.t("zoom")).clicked() {
                self.zoom = (self.zoom * 1.25).clamp(0.25, 8.0);
            }
            ui.separator();
            if icons::icon_button(ui, Icon::Hand, self.pan_tool, self.t("pan_tool")).clicked() {
                self.pan_tool = !self.pan_tool;
            }
            if icons::icon_button(ui, Icon::Compare, self.compare_on, self.t("compare_tool"))
                .clicked()
            {
                self.compare_on = !self.compare_on;
            }
        });
    }

    fn queue_strip(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        theme::card_frame().show(ui, |ui| {
            ui.horizontal(|ui| {
                icons::paint(
                    ui,
                    Icon::Queue,
                    ui.next_widget_position() + egui::vec2(8.0, 10.0),
                    18.0,
                    theme::TEXT,
                );
                ui.add_space(24.0);
                ui.label(theme::section_text(self.t("image_queue")));
                ui.label(theme::caption_text(
                    self.tr("queue_count_short", &[&self.queue.len().to_string()]),
                ));
            });
            if self.queue.is_empty() {
                ui.label(theme::caption_text(self.t("queue_empty")));
            } else {
                egui::ScrollArea::horizontal()
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            let mut select = None;
                            for (index, item) in self.queue.iter().enumerate() {
                                let selected = index == self.queue_selected;
                                let (rect, response) = ui.allocate_exact_size(
                                    egui::vec2(108.0, 104.0),
                                    egui::Sense::click(),
                                );
                                let painter = ui.painter_at(rect);
                                let border_color =
                                    if selected || matches!(item.status, QueueStatus::Processing) {
                                        theme::ACCENT
                                    } else {
                                        theme::BORDER
                                    };
                                painter.rect(
                                    rect,
                                    8.0,
                                    theme::CARD,
                                    egui::Stroke::new(
                                        if selected
                                            || matches!(item.status, QueueStatus::Processing)
                                        {
                                            2.0
                                        } else {
                                            1.0
                                        },
                                        border_color,
                                    ),
                                    egui::StrokeKind::Inside,
                                );
                                if let Some(thumb) = self.thumbs.get(&item.path) {
                                    let id = match item.status {
                                        QueueStatus::Pending => thumb.gray.id(),
                                        _ => thumb.color.id(),
                                    };
                                    let thumb_size = match item.status {
                                        QueueStatus::Pending => thumb.gray.size_vec2(),
                                        _ => thumb.color.size_vec2(),
                                    };
                                    let thumb_aspect = if thumb_size.y > 0.0 {
                                        thumb_size.x / thumb_size.y
                                    } else {
                                        1.0
                                    };
                                    let image_rect = fit_rect(
                                        egui::Rect::from_center_size(
                                            egui::pos2(rect.center().x, rect.top() + 40.0),
                                            egui::vec2(64.0, 70.0),
                                        ),
                                        thumb_aspect,
                                    );
                                    painter.image(
                                        id,
                                        image_rect,
                                        egui::Rect::from_min_max(
                                            egui::pos2(0.0, 0.0),
                                            egui::pos2(1.0, 1.0),
                                        ),
                                        egui::Color32::WHITE,
                                    );
                                } else if self.thumbs.is_loading(&item.path) {
                                    icons::paint_spinner(
                                        ui,
                                        egui::pos2(rect.center().x, rect.top() + 38.0),
                                        10.0,
                                        theme::ACCENT,
                                    );
                                }
                                painter.text(
                                    egui::pos2(rect.center().x, rect.bottom() - 12.0),
                                    egui::Align2::CENTER_CENTER,
                                    (index + 1).to_string(),
                                    egui::FontId::proportional(12.0),
                                    theme::TEXT,
                                );
                                let badge = match item.status {
                                    QueueStatus::Done { .. } => Some((Icon::Check, theme::SUCCESS)),
                                    QueueStatus::Failed { .. } => {
                                        Some((Icon::Warning, theme::ERROR))
                                    }
                                    QueueStatus::Processing => Some((Icon::Refresh, theme::ACCENT)),
                                    _ if selected => Some((Icon::Check, theme::ACCENT)),
                                    _ => None,
                                };
                                if let Some((icon, color)) = badge {
                                    painter.circle_filled(
                                        rect.right_bottom() - egui::vec2(15.0, 15.0),
                                        10.0,
                                        theme::WHITE,
                                    );
                                    icons::paint(
                                        ui,
                                        icon,
                                        rect.right_bottom() - egui::vec2(15.0, 15.0),
                                        15.0,
                                        color,
                                    );
                                }
                                if item.path.file_name().is_some() {
                                    response.clone().on_hover_text(
                                        item.path.file_name().unwrap().to_string_lossy(),
                                    );
                                }
                                if response.clicked() {
                                    select = Some(index);
                                }
                            }
                            if let Some(index) = select {
                                self.select_queue_item(ctx, index);
                            }
                        });
                    });
            }
        });
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
