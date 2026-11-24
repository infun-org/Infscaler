use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use anyhow::{Context, Result};

use crate::engine::{self, JobOptions};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputMode {
    Sharpen,
    Upscale2,
    Upscale3,
    Upscale4,
}

#[derive(Debug, Clone)]
pub struct BatchOptions {
    pub input_dir: PathBuf,
    pub input_files: Option<Vec<PathBuf>>,
    pub output_dir: Option<PathBuf>,
    pub jobs: usize,
    pub threads: usize,
    pub mode: OutputMode,
    pub preset: String,
}

#[derive(Debug, Default)]
pub struct BatchControl {
    pub paused: AtomicBool,
    pub cancelled: AtomicBool,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub enum BatchEvent {
    Line(String),
    FileStarted {
        index: usize,
        total: usize,
        path: PathBuf,
    },
    FileFinished {
        index: usize,
        total: usize,
        path: PathBuf,
        output: PathBuf,
    },
    FileSkipped {
        index: usize,
        total: usize,
        path: PathBuf,
        output: PathBuf,
    },
    FileFailed {
        index: usize,
        total: usize,
        path: PathBuf,
        error: String,
    },
}

pub fn scan_folder(folder: &Path) -> Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    for entry in std::fs::read_dir(folder).with_context(|| format!("scan {}", folder.display()))? {
        let entry = entry?;
        let path = entry.path();
        if path.is_file() && engine::is_image_path(&path) {
            files.push(path);
        }
    }
    files.sort();
    Ok(files)
}

pub fn default_output_dir(input: &Path) -> PathBuf {
    let name = input.file_name().unwrap_or_default();
    input.with_file_name(format!("{}_infscaler", name.to_string_lossy()))
}

pub fn output_path(output_dir: &Path, input: &Path) -> PathBuf {
    output_dir.join(input.file_name().unwrap_or_default())
}

pub fn should_skip(output: &Path) -> bool {
    output.is_file()
}

fn commit_temp(temp: &Path, output: &Path) -> std::io::Result<()> {
    std::fs::rename(temp, output)
}

pub fn run(
    root: &Path,
    options: &BatchOptions,
    control: Arc<BatchControl>,
    report: impl FnMut(BatchEvent) + Send + 'static,
) -> Result<()> {
    let files = options
        .input_files
        .clone()
        .unwrap_or(scan_folder(&options.input_dir)?);
    let output_dir = options
        .output_dir
        .clone()
        .unwrap_or_else(|| default_output_dir(&options.input_dir));
    std::fs::create_dir_all(&output_dir)?;
    let requested_jobs = options.jobs.clamp(1, 3);
    let jobs = requested_jobs.min(options.threads.max(1));
    let threads = options.threads.max(1) / jobs;
    crate::ort_backend::set_thread_count(threads.max(1));
    let reporter = Arc::new(Mutex::new(
        Box::new(report) as Box<dyn FnMut(BatchEvent) + Send>
    ));
    report_line(
        &reporter,
        format!(
            "{} images; {} worker(s){}; {} ONNX thread(s) each",
            files.len(),
            jobs,
            if jobs < requested_jobs {
                format!(" (limited from {requested_jobs} to keep the thread budget bounded)")
            } else {
                String::new()
            },
            threads.max(1)
        ),
    );
    let total = options
        .input_files
        .as_ref()
        .map(Vec::len)
        .unwrap_or(files.len());
    let queue = Arc::new(Mutex::new(files.into_iter().enumerate()));
    let done = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let errors = Arc::new(Mutex::new(Vec::<String>::new()));
    let started = Instant::now();
    let mut workers = Vec::new();
    for worker in 0..jobs {
        let queue = queue.clone();
        let done = done.clone();
        let errors = errors.clone();
        let ctl = control.clone();
        let root = root.to_path_buf();
        let opts = options.clone();
        let out_dir = output_dir.clone();
        let reporter = reporter.clone();
        workers.push(thread::spawn(move || {
            let _ = std::process::Command::new("renice")
                .args(["-n", "10", "-p", &std::process::id().to_string()])
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .status();
            loop {
                if ctl.cancelled.load(Ordering::SeqCst) {
                    break;
                }
                while ctl.paused.load(Ordering::SeqCst) && !ctl.cancelled.load(Ordering::SeqCst) {
                    thread::sleep(Duration::from_millis(250));
                }
                if ctl.cancelled.load(Ordering::SeqCst) {
                    break;
                }
                wait_for_memory(&ctl);
                let next = queue.lock().ok().and_then(|mut q| q.next());
                let Some((index, input)) = next else { break };
                let output = output_path(&out_dir, &input);
                if should_skip(&output) {
                    report_line(
                        &reporter,
                        format!(
                            "{}/{} skipped existing output: {}",
                            index + 1,
                            total,
                            output.display()
                        ),
                    );
                    report_event(
                        &reporter,
                        BatchEvent::FileSkipped {
                            index,
                            total,
                            path: input.clone(),
                            output: output.clone(),
                        },
                    );
                    done.fetch_add(1, Ordering::SeqCst);
                    continue;
                }
                let temp = temp_output_path(&output, worker);
                if let Ok((w, h)) = image::image_dimensions(&input) {
                    let estimate_kb = (w as u64).saturating_mul(h as u64).saturating_mul(64) / 1024
                        + if opts.preset == "people" {
                            2_200_000
                        } else {
                            700_000
                        };
                    let available_kb = available_memory_kb();
                    if estimate_kb.saturating_add(1_500_000) > available_kb {
                        report_line(
                            &reporter,
                            format!(
                                "Warning: {} may need about {} MiB; MemAvailable is {} MiB",
                                input.display(),
                                estimate_kb / 1024,
                                available_kb / 1024
                            ),
                        );
                    }
                }
                let mode = opts.mode;
                let mut job = JobOptions {
                    input: input.clone(),
                    output: temp.clone(),
                    model: "auto".into(),
                    output_scale: match mode {
                        OutputMode::Upscale2 => 2,
                        OutputMode::Upscale3 => 3,
                        _ => 4,
                    },
                    tile: 0,
                    tta: false,
                    keep_size: mode == OutputMode::Sharpen,
                    denoise: false,
                    overlap: 16,
                    gpu: crate::engine::GpuChoice::Auto,
                    face_restore: false,
                    face_strength: 0.0,
                    face_model: "RestoreFormerPlusPlus".into(),
                    fast: false,
                    preset: opts.preset.clone(),
                };
                let cancel = AtomicBool::new(false);
                let pid = AtomicU32::new(0);
                let image_started = Instant::now();
                report_event(
                    &reporter,
                    BatchEvent::FileStarted {
                        index,
                        total,
                        path: input.clone(),
                    },
                );
                let mut result = engine::run_job(&root, &job, &cancel, &pid, |_| {});
                if result
                    .as_ref()
                    .err()
                    .is_some_and(|e| format!("{e:#}").contains("NCNN produced an all-black"))
                {
                    report_line(
                        &reporter,
                        format!(
                            "{}: NCNN output was black; retrying with Nomos2 ONNX",
                            input.display()
                        ),
                    );
                    let _ = std::fs::remove_file(&temp);
                    job.model = "4xNomos2_otf_esrgan".into();
                    job.preset = "general".into();
                    result = engine::run_job(&root, &job, &cancel, &pid, |_| {});
                }
                let message = match result {
                    Ok(outcome) => {
                        report_line(
                            &reporter,
                            format!(
                                "METRIC,{}, {:.3}, {}, {}",
                                input.display(),
                                outcome.elapsed.as_secs_f64(),
                                outcome.peak_rss_kb,
                                outcome.backend
                            ),
                        );
                        commit_temp(&temp, &output).map(|_| {
                            format!(
                                "{}/{} done in {:.1}s",
                                index + 1,
                                total,
                                outcome.elapsed.as_secs_f32()
                            )
                        })
                    }
                    Err(err) => {
                        let _ = std::fs::remove_file(&temp);
                        Err(std::io::Error::other(format!("{err:#}")))
                    }
                };
                match &message {
                    Ok(_) => report_event(
                        &reporter,
                        BatchEvent::FileFinished {
                            index,
                            total,
                            path: input.clone(),
                            output: output.clone(),
                        },
                    ),
                    Err(err) => {
                        if let Ok(mut e) = errors.lock() {
                            e.push(format!("{}: {err}", input.display()));
                        }
                        report_event(
                            &reporter,
                            BatchEvent::FileFailed {
                                index,
                                total,
                                path: input.clone(),
                                error: err.to_string(),
                            },
                        );
                    }
                }
                let completed = done.fetch_add(1, Ordering::SeqCst) + 1;
                let elapsed = started.elapsed();
                let eta = if completed > 0 {
                    elapsed.mul_f64((total.saturating_sub(completed)) as f64 / completed as f64)
                } else {
                    Duration::ZERO
                };
                report_line(
                    &reporter,
                    format!(
                        "{completed}/{total} {} {:.1}s; ETA {:.1}s",
                        input.file_name().unwrap_or_default().to_string_lossy(),
                        image_started.elapsed().as_secs_f32(),
                        eta.as_secs_f32()
                    ),
                );
            }
        }));
    }
    for worker in workers {
        let _ = worker.join();
    }
    if !errors.lock().unwrap().is_empty() {
        let log = output_dir.join("infscaler-batch.log");
        std::fs::write(&log, errors.lock().unwrap().join("\n"))?;
        report_line(
            &reporter,
            format!(
                "{} failed image(s); details: {}",
                errors.lock().unwrap().len(),
                log.display()
            ),
        );
    }
    report_line(
        &reporter,
        format!(
            "Batch finished: {}/{} images in {:.1}s",
            done.load(Ordering::Relaxed),
            total,
            started.elapsed().as_secs_f32()
        ),
    );
    Ok(())
}

fn report_event(reporter: &Arc<Mutex<Box<dyn FnMut(BatchEvent) + Send>>>, event: BatchEvent) {
    if let Ok(mut report) = reporter.lock() {
        report(event);
    }
}

fn report_line(reporter: &Arc<Mutex<Box<dyn FnMut(BatchEvent) + Send>>>, message: String) {
    report_event(reporter, BatchEvent::Line(message));
}

fn temp_output_path(output: &Path, worker: usize) -> PathBuf {
    let stem = output.file_stem().unwrap_or_default().to_string_lossy();
    let ext = output.extension().unwrap_or_default().to_string_lossy();
    output.with_file_name(format!(".{stem}.infscaler-{worker}.tmp.{ext}"))
}

fn wait_for_memory(control: &BatchControl) {
    while !control.cancelled.load(Ordering::SeqCst) {
        let available = available_memory_kb();
        if available >= 2_000_000 {
            break;
        }
        thread::sleep(Duration::from_secs(3));
    }
}

fn available_memory_kb() -> u64 {
    let host = std::fs::read_to_string("/proc/meminfo")
        .ok()
        .and_then(|s| {
            s.lines()
                .find(|l| l.starts_with("MemAvailable:"))
                .and_then(|l| l.split_whitespace().nth(1)?.parse::<u64>().ok())
        })
        .unwrap_or(u64::MAX);
    let max = std::fs::read_to_string("/sys/fs/cgroup/memory.max")
        .ok()
        .and_then(|s| s.trim().parse::<u64>().ok());
    let current = std::fs::read_to_string("/sys/fs/cgroup/memory.current")
        .ok()
        .and_then(|s| s.trim().parse::<u64>().ok());
    match (max, current) {
        (Some(max), Some(current)) => host.min(max.saturating_sub(current) / 1024),
        _ => host,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn scan_only_top_level_and_filter_extensions() {
        let d = std::env::temp_dir().join(format!("infscaler-scan-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(d.join("nested")).unwrap();
        for f in ["a.JPG", "b.webp", "c.txt"] {
            std::fs::write(d.join(f), b"").unwrap();
        }
        std::fs::write(d.join("nested/hidden.png"), b"").unwrap();
        assert_eq!(scan_folder(&d).unwrap().len(), 2);
        let _ = std::fs::remove_dir_all(&d);
    }
    #[test]
    fn output_naming_and_resume_path() {
        assert_eq!(
            output_path(Path::new("out"), Path::new("a.jpg")),
            PathBuf::from("out/a.jpg")
        );
        let existing =
            std::env::temp_dir().join(format!("infscaler-resume-{}", std::process::id()));
        std::fs::write(&existing, b"already done").unwrap();
        assert!(should_skip(&existing));
        let _ = std::fs::remove_file(existing);
        assert!(!should_skip(Path::new("/definitely/missing")));
    }
    #[test]
    fn temp_rename_is_atomic_file_commit() {
        let d = std::env::temp_dir().join(format!("infscaler-rename-{}", std::process::id()));
        std::fs::create_dir_all(&d).unwrap();
        let a = d.join("tmp");
        let b = d.join("final");
        std::fs::write(&a, b"ready").unwrap();
        commit_temp(&a, &b).unwrap();
        assert_eq!(std::fs::read(&b).unwrap(), b"ready");
        assert!(!a.exists());
        let _ = std::fs::remove_dir_all(d);
    }
}
