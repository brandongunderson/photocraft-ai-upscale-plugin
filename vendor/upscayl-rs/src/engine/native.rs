use crate::config::UpscaylOptions;
use std::io::{self, Read};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    mpsc,
};
use std::time::{Duration, Instant};

pub struct NativeNcnn;

impl NativeNcnn {
    pub fn find_binary() -> Option<PathBuf> {
        if let Some(p) = std::env::var_os("UPSCAYL_BIN")
            .map(PathBuf::from)
            .filter(|p| p.is_file())
        {
            return Some(p);
        }
        let name = if cfg!(windows) {
            "upscayl-bin.exe"
        } else {
            "upscayl-bin"
        };
        std::env::var_os("PATH").and_then(|p| {
            std::env::split_paths(&p)
                .map(|d| d.join(name))
                .find(|p| p.is_file())
        })
    }

    pub fn execute(
        binary: &Path,
        input: &Path,
        output: &Path,
        options: &UpscaylOptions,
        progress: Option<&dyn Fn(f32)>,
    ) -> io::Result<()> {
        Self::execute_cancellable(
            binary,
            input,
            output,
            options,
            progress,
            &AtomicBool::new(false),
        )
    }

    /// Local patch: bounded CR/LF diagnostics, drained while polling so cancellation can reap the child.
    pub fn execute_cancellable(
        binary: &Path,
        input: &Path,
        output: &Path,
        options: &UpscaylOptions,
        progress: Option<&dyn Fn(f32)>,
        cancel: &AtomicBool,
    ) -> io::Result<()> {
        let mut cmd = Command::new(binary);
        cmd.arg("-i")
            .arg(input)
            .arg("-o")
            .arg(output)
            .arg("-n")
            .arg(&options.model);
        if let Some(models) = &options.models_path {
            cmd.arg("-m").arg(models);
        }
        if let Some(s) = options.scale {
            cmd.arg("-s").arg(s.to_string());
        }
        if let Some(t) = options.tile_size.filter(|v| *v > 0) {
            cmd.arg("-t").arg(t.to_string());
        }
        if let Some(g) = options.gpu_id {
            cmd.arg("-g").arg(g.to_string());
        }
        cmd.arg("-f").arg(options.format.extension());
        if options.tta_mode {
            cmd.arg("-x");
        }
        cmd.stdout(Stdio::null()).stderr(Stdio::piped());
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            cmd.creation_flags(0x08000000);
        }
        let mut child = cmd.spawn()?;
        let Some(mut stderr) = child.stderr.take() else {
            let _ = child.kill();
            let _ = child.wait();
            return Err(io::Error::other("AI runtime has no diagnostics stream."));
        };
        let (send, recv) = mpsc::sync_channel::<String>(64);
        let drain = std::thread::spawn(move || {
            let mut buf = [0u8; 1024];
            let mut line = Vec::with_capacity(2048);
            loop {
                let n = match stderr.read(&mut buf) {
                    Ok(0) | Err(_) => break,
                    Ok(n) => n,
                };
                for &b in buf.iter().take(n) {
                    if b == b'\n' || b == b'\r' {
                        if !line.is_empty() {
                            let _ = send.try_send(String::from_utf8_lossy(&line).into_owned());
                            line.clear();
                        }
                    } else if line.len() < 2048 {
                        line.push(b);
                    }
                }
            }
            if !line.is_empty() {
                let _ = send.try_send(String::from_utf8_lossy(&line).into_owned());
            }
        });
        let started = Instant::now();
        let mut tail = std::collections::VecDeque::new();
        loop {
            for line in recv.try_iter() {
                if let Some(cb) = progress {
                    if let Some(before) = line.split('%').next() {
                        if let Some(number) = before
                            .split_whitespace()
                            .last()
                            .and_then(|s| s.parse::<f32>().ok())
                            .filter(|v| v.is_finite())
                        {
                            cb(number);
                        }
                    }
                }
                if tail.len() == 12 {
                    tail.pop_front();
                }
                tail.push_back(line);
            }
            if cancel.load(Ordering::Relaxed) || started.elapsed() > Duration::from_secs(1800) {
                let _ = child.kill();
                let _ = child.wait();
                let _ = drain.join();
                return Err(io::Error::other(if cancel.load(Ordering::Relaxed) {
                    "Cancelled. No output was saved."
                } else {
                    "AI runtime timed out after 30 minutes."
                }));
            }
            match child.try_wait() {
                Ok(Some(status)) => {
                    let _ = drain.join();
                    for line in recv.try_iter() {
                        if tail.len() == 12 {
                            tail.pop_front();
                        }
                        tail.push_back(line);
                    }
                    if !status.success() {
                        return Err(io::Error::other(format!("AI runtime failed ({status}). Check the Vulkan driver, GPU ID, and tile size.\n{}", tail.into_iter().collect::<Vec<_>>().join("\n"))));
                    }
                    if let Some(cb) = progress {
                        cb(100.0);
                    }
                    return Ok(());
                }
                Ok(None) => std::thread::sleep(Duration::from_millis(40)),
                Err(e) => {
                    let _ = child.kill();
                    let _ = child.wait();
                    let _ = drain.join();
                    return Err(e);
                }
            }
        }
    }
}
