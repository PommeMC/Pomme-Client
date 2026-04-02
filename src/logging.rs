use std::path::Path;
use tracing_subscriber::prelude::*;

pub fn init(log_dir: &Path) {
    let file_appender = tracing_appender::rolling::never(log_dir, "latest.log");
    let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);

    Box::leak(Box::new(guard));

    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::from_default_env())
        .with(
            tracing_subscriber::fmt::layer()
                .with_ansi(false)
                .with_writer(non_blocking),
        )
        .with(tracing_subscriber::fmt::layer().with_writer(std::io::stdout))
        .init();
}

pub fn rotate(log_dir: &Path) -> std::io::Result<()> {
    let latest = log_dir.join("latest.log");
    if !latest.exists() {
        return Ok(());
    }
    let modified = latest.metadata()?.modified()?;
    let datetime = chrono::DateTime::<chrono::Local>::from(modified);
    let date = datetime.format("%Y-%m-%d");
    let index = (1..)
        .find(|i| !log_dir.join(format!("{date}-{i}.log.gz")).exists())
        .unwrap();
    let dest = log_dir.join(format!("{date}-{index}.log.gz"));
    let input = std::fs::read(&latest)?;
    let output_file = std::fs::File::create(&dest)?;
    let mut encoder = flate2::write::GzEncoder::new(output_file, flate2::Compression::default());
    std::io::Write::write_all(&mut encoder, &input)?;
    encoder.finish().map_err(std::io::Error::other)?;
    std::fs::remove_file(&latest)?;
    Ok(())
}
