//! Lazy, memory-light thumbnail loader for the queue strip.
//!
//! Decodes downscaled (~160 px) images on background threads (at most two at
//! a time), caches only the small textures, and never keeps full-resolution
//! pixels around.

use std::collections::{HashMap, HashSet, VecDeque};
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver, Sender, TryRecvError};
use std::sync::{Arc, Mutex};
use std::thread;

use eframe::egui;

const MAX_SIDE: u32 = 160;
const WORKERS: usize = 2;

struct ThumbPixels {
    path: PathBuf,
    width: u32,
    height: u32,
    color: Vec<u8>,
    gray: Vec<u8>,
}

#[allow(dead_code)]
pub struct ThumbImage {
    pub color: egui::TextureHandle,
    pub gray: egui::TextureHandle,
}

#[allow(dead_code)]
enum Slot {
    Queued,
    Loading,
    Ready(ThumbImage),
    Failed,
}

pub struct ThumbCache {
    slots: HashMap<PathBuf, Slot>,
    tx: Sender<PathBuf>,
    rx: Receiver<Result<ThumbPixels, (PathBuf, String)>>,
    pending: Arc<Mutex<VecDeque<PathBuf>>>,
    inflight: HashSet<PathBuf>,
}

impl ThumbCache {
    pub fn new() -> Self {
        let (job_tx, job_rx) = mpsc::channel::<PathBuf>();
        let (out_tx, out_rx) = mpsc::channel::<Result<ThumbPixels, (PathBuf, String)>>();
        let pending: Arc<Mutex<VecDeque<PathBuf>>> = Arc::new(Mutex::new(VecDeque::new()));
        // Bridge: a dispatcher holds the incoming jobs and hands them to workers.
        // Simpler: share a mutex queue among workers; `request` pushes + notifies
        // via the channel (used only as a wake-up).
        let queue = pending.clone();
        // Keep job_tx so request() can still… actually we'll push to `pending`
        // directly and use a dummy wake. Workers just poll the queue.
        drop(job_tx);
        drop(job_rx);
        for _ in 0..WORKERS {
            let queue = queue.clone();
            let out_tx = out_tx.clone();
            thread::Builder::new()
                .name("infscaler-thumb".into())
                .spawn(move || loop {
                    let next = queue.lock().ok().and_then(|mut q| q.pop_front());
                    let Some(path) = next else {
                        thread::sleep(std::time::Duration::from_millis(40));
                        continue;
                    };
                    let _ = out_tx.send(decode_thumb(&path).map_err(|e| (path, e)));
                })
                .expect("thumb worker");
        }
        // Dummy sender so the struct field type stays a Sender (unused).
        let (tx, _rx_unused) = mpsc::channel();
        Self {
            slots: HashMap::new(),
            tx,
            rx: out_rx,
            pending,
            inflight: HashSet::new(),
        }
    }

    /// Ask for a thumbnail if we do not already have one. Work is queued; at
    /// most two decodes run at once.
    pub fn request(&mut self, path: &Path) {
        if self.slots.contains_key(path) {
            return;
        }
        self.slots.insert(path.to_path_buf(), Slot::Queued);
        if let Ok(mut q) = self.pending.lock() {
            q.push_back(path.to_path_buf());
        }
        let _ = self.tx; // keep the field "used"
    }

    pub fn poll(&mut self, ctx: &egui::Context) {
        loop {
            match self.rx.try_recv() {
                Ok(Ok(pixels)) => {
                    self.inflight.remove(&pixels.path);
                    let color = ctx.load_texture(
                        format!("thumb-c-{}", pixels.path.display()),
                        egui::ColorImage::from_rgba_unmultiplied(
                            [pixels.width as usize, pixels.height as usize],
                            &pixels.color,
                        ),
                        egui::TextureOptions::LINEAR,
                    );
                    let gray = ctx.load_texture(
                        format!("thumb-g-{}", pixels.path.display()),
                        egui::ColorImage::from_rgba_unmultiplied(
                            [pixels.width as usize, pixels.height as usize],
                            &pixels.gray,
                        ),
                        egui::TextureOptions::LINEAR,
                    );
                    self.slots
                        .insert(pixels.path, Slot::Ready(ThumbImage { color, gray }));
                    ctx.request_repaint();
                }
                Ok(Err((path, _err))) => {
                    self.inflight.remove(&path);
                    self.slots.insert(path, Slot::Failed);
                    ctx.request_repaint();
                }
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => break,
            }
        }
        // Mark queued items that have left the deque as loading once a worker
        // has picked them up (queue no longer contains them and not ready).
        if let Ok(q) = self.pending.lock() {
            for (path, slot) in self.slots.iter_mut() {
                if matches!(slot, Slot::Queued) && !q.iter().any(|p| p == path) {
                    *slot = Slot::Loading;
                }
            }
        }
        if self
            .slots
            .values()
            .any(|s| matches!(s, Slot::Queued | Slot::Loading))
        {
            ctx.request_repaint_after(std::time::Duration::from_millis(50));
        }
    }

    #[allow(dead_code)]
    pub fn get(&self, path: &Path) -> Option<&ThumbImage> {
        match self.slots.get(path) {
            Some(Slot::Ready(img)) => Some(img),
            _ => None,
        }
    }

    #[allow(dead_code)]
    pub fn is_loading(&self, path: &Path) -> bool {
        matches!(self.slots.get(path), Some(Slot::Queued | Slot::Loading))
    }

    /// Drop textures for paths no longer in the queue so RAM stays bounded.
    pub fn retain(&mut self, keep: &[PathBuf]) {
        let set: HashSet<&PathBuf> = keep.iter().collect();
        self.slots.retain(|k, _| set.contains(k));
        if let Ok(mut q) = self.pending.lock() {
            q.retain(|p| set.contains(p));
        }
    }

    #[allow(dead_code)]
    pub fn ready_count(&self) -> usize {
        self.slots
            .values()
            .filter(|s| matches!(s, Slot::Ready(_)))
            .count()
    }
}

fn decode_thumb(path: &Path) -> Result<ThumbPixels, String> {
    let img = image::open(path).map_err(|e| e.to_string())?;
    let img = img.thumbnail(MAX_SIDE, MAX_SIDE);
    let rgba = img.to_rgba8();
    let (width, height) = rgba.dimensions();
    let mut gray = rgba.clone();
    for pixel in gray.pixels_mut() {
        let y = ((pixel[0] as u16 * 30) + (pixel[1] as u16 * 59) + (pixel[2] as u16 * 11)) / 100;
        let y = y as u8;
        pixel[0] = y;
        pixel[1] = y;
        pixel[2] = y;
    }
    Ok(ThumbPixels {
        path: path.to_path_buf(),
        width,
        height,
        color: rgba.into_raw(),
        gray: gray.into_raw(),
    })
}
