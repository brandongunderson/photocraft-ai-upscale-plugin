use std::io::Write;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().collect();
    let arg = |flag: &str| {
        args.windows(2)
            .find(|a| a.first().is_some_and(|v| v == flag))
            .and_then(|a| a.get(1))
    };
    if arg("-n").is_some_and(|s| s == "sleep") {
        std::thread::sleep(std::time::Duration::from_secs(60));
        return Ok(());
    }
    if arg("-n").is_some_and(|s| s == "fail") {
        eprintln!("Synthetic GPU failure");
        std::process::exit(3);
    }
    let input = arg("-i").ok_or("Missing input")?;
    let output = arg("-o").ok_or("Missing output")?;
    if arg("-m").is_some_and(|path| path.encode_utf16().count() >= 240) {
        return Err("Synthetic legacy model path buffer overflow".into());
    }
    let root = std::env::current_exe()?
        .parent()
        .ok_or("No executable folder")?
        .to_path_buf();
    if root.join("sleep-worker").is_file() {
        std::fs::write(root.join("worker-started"), b"ready")?;
        std::thread::sleep(std::time::Duration::from_secs(60));
        return Ok(());
    }
    if root.join("fail-second-tile").is_file() {
        let count_path = root.join("invocations");
        let count = std::fs::read_to_string(&count_path)
            .ok()
            .and_then(|s| s.parse::<u32>().ok())
            .unwrap_or(0)
            + 1;
        std::fs::write(count_path, count.to_string())?;
        if count > 1 {
            eprintln!("Synthetic save failed with successful exit status");
            return Ok(());
        }
    }
    // Enough unread stdout to deadlock the original wrapper; ours sends stdout to null.
    std::io::stdout().write_all(&vec![b'x'; 1024 * 1024])?;
    std::io::stderr().write_all(b"10.0%\r50.0%\r100.0%\n")?;
    let image = image::open(input)?;
    image
        .resize_exact(
            image.width() * 4,
            image.height() * 4,
            image::imageops::FilterType::Nearest,
        )
        .save(output)?;
    Ok(())
}
