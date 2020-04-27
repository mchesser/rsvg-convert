use std::{env, fs, io::Write, path::PathBuf, process::Command};

use anyhow::Context;
use structopt::StructOpt;

#[derive(Debug, StructOpt)]
#[structopt(name = "example", about = "An example of StructOpt usage.")]
struct Opt {
    /// Set the X resolution of the image in pixels per inch.
    #[structopt(short = "d", long = "dpi-x", default_value = "90")]
    dpi_x: f64,

    /// Set the Y resolution of the image in pixels per inch.
    #[structopt(short = "p", long = "dpi-y", default_value = "90")]
    dpi_y: f64,

    /// X Zoom factor, as a percentage
    #[structopt(short = "x", long = "x-zoom", default_value = "1.0")]
    x_zoom: f64,

    /// Y Zoom factor, as a percentage
    #[structopt(short = "y", long = "y-zoom", default_value = "1.0")]
    y_zoom: f64,

    /// Specify how wide you wish the image to be. If unspecified, the natural width of the image
    /// is used as the default.
    #[structopt(short, long)]
    width: Option<u64>,

    /// Specify how tall you wish the image to be. If unspecified, the natural width of the image
    /// is used as the default.
    #[structopt(short, long)]
    height: Option<u64>,

    /// Specify the output format you wish the image to be saved in. If unspecified, PNG is used
    /// as the default.
    #[structopt(short, long, default_value = "png")]
    format: String,

    /// Specify that the aspect ratio is to be preserved. If unspecified, aspect ratio will not be
    /// preserved.
    #[structopt(short = "a", long = "keep-aspect-ratio")]
    keep_aspect_ratio: bool,

    /// Input file
    #[structopt(parse(from_os_str))]
    input: PathBuf,

    /// Output file, stdout if not present
    #[structopt(long, short, parse(from_os_str))]
    output: Option<PathBuf>,
}

fn hash_input(opt: &Opt) -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let mut hasher = DefaultHasher::new();
    ((opt.dpi_x * 1000.0) as u64).hash(&mut hasher);
    ((opt.dpi_y * 1000.0) as u64).hash(&mut hasher);
    ((opt.x_zoom * 1000.0) as u64).hash(&mut hasher);
    ((opt.y_zoom * 1000.0) as u64).hash(&mut hasher);
    opt.width.hash(&mut hasher);
    opt.height.hash(&mut hasher);
    opt.format.hash(&mut hasher);
    opt.keep_aspect_ratio.hash(&mut hasher);

    // TODO: consider hashing the contents of the file instead of the filename, however this is not
    // a huge issue since pandoc already does this.
    opt.input.file_name().hash(&mut hasher);

    format!("{:0x}", hasher.finish())
}

fn main() -> Result<(), anyhow::Error> {
    let opt = Opt::from_args();

    let mut cache_dir = env::temp_dir();
    cache_dir.push("rsvg-convert-cache");

    if !cache_dir.exists() {
        fs::create_dir(&cache_dir).context("failed to create temporary directory")?;
    }

    let mut cached_file = cache_dir;
    cached_file.push(hash_input(&opt));
    cached_file.set_extension(&opt.format);

    if !cached_file.exists() {
        eprintln!("Converting file: {:?}", opt.input);
        let mut cmd = Command::new("inkscape");
        cmd.arg(&opt.input).arg("--without-gui");

        match opt.format.as_str() {
            "png" => cmd.arg("-e"),
            "pdf" => cmd.arg("-A"),
            "ps" => cmd.arg("-P"),
            "eps" => cmd.arg("-E"),
            "wmf" => cmd.arg("-m"),
            "emf" => cmd.arg("-M"),
            x => return Err(anyhow::anyhow!("Unsupported file format: {}", x)),
        };
        cmd.arg(&cached_file);

        assert_eq!(
            opt.dpi_x, opt.dpi_y,
            "Different DPI values for x and y currently not supported"
        );
        cmd.arg("--export-dpi").arg(opt.dpi_x.to_string());

        // TODO: check for `keep aspect ratio` and update width and height appropriately
        if let Some(width) = opt.width {
            cmd.arg("--export-width").arg(width.to_string());
        }
        if let Some(height) = opt.height {
            cmd.arg("--export-height").arg(height.to_string());
        }

        eprintln!("Running: {:?}", cmd);
        let output = cmd.output().context("Failed to execute inkscape")?;

        if !output.status.success() {
            let error = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow::anyhow!("Inkscape error:\n{}", error));
        }

        let _ = std::io::stdout().write_all(&output.stdout);
        let _ = std::io::stderr().write_all(&output.stderr);
    }
    else {
        eprintln!("Loading from cache: {:?}", cached_file);
    }

    if let Some(output) = opt.output {
        fs::copy(cached_file, output).context("Failed to copy output to destination")?;
    }
    else {
        let mut data = std::fs::File::open(cached_file).context("No output was generated")?;
        std::io::copy(&mut data, &mut std::io::stdout().lock())
            .context("Failed to write output to stdout")?;
    }

    Ok(())
}
