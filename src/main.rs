use std::{env, fs, io::Write, path::PathBuf, process::Command};

use anyhow::Context;
use clap::Parser;

#[derive(Debug, clap::Parser)]
#[command(disable_help_flag = true)]
struct Args {
    /// Display this message.
    #[clap(long, action = clap::ArgAction::Help)]
    help: Option<bool>,

    /// Set the X resolution of the image in pixels per inch.
    #[clap(short = 'd', long = "dpi-x", default_value = "90")]
    dpi_x: f64,

    /// Set the Y resolution of the image in pixels per inch.
    #[clap(short = 'p', long = "dpi-y", default_value = "90")]
    dpi_y: f64,

    /// X Zoom factor, as a percentage
    #[clap(short = 'x', long = "x-zoom", default_value = "1.0")]
    x_zoom: f64,

    /// Y Zoom factor, as a percentage
    #[clap(short = 'y', long = "y-zoom", default_value = "1.0")]
    y_zoom: f64,

    /// Specify how wide you wish the image to be. If unspecified, the natural width of the image
    /// is used as the default.
    #[clap(short, long)]
    width: Option<u64>,

    /// Specify how tall you wish the image to be. If unspecified, the natural width of the image
    /// is used as the default.
    #[clap(short, long)]
    height: Option<u64>,

    /// Specify the output format you wish the image to be saved in. If unspecified, PNG is used
    /// as the default.
    #[clap(short, long, default_value = "png")]
    format: String,

    /// Specify that the aspect ratio is to be preserved. If unspecified, aspect ratio will not be
    /// preserved.
    #[clap(short = 'a', long = "keep-aspect-ratio")]
    keep_aspect_ratio: bool,

    /// Input file, stdin if not present.
    input: Option<PathBuf>,

    /// Output file, stdout if not present.
    #[clap(long, short)]
    output: Option<PathBuf>,
}

fn hash_input(opt: &Args) -> Option<String> {
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

    // Note: If the input is from `stdin` then never use a cached image.
    // TODO: consider hashing the contents of the file instead of the filename, however this is not
    // a huge issue since pandoc already does this.
    let path = opt.input.as_ref()?;
    path.hash(&mut hasher);

    Some(format!("{:0x}", hasher.finish()))
}

fn main() -> anyhow::Result<()> {
    eprintln!("{}", std::env::args().collect::<Vec<String>>().join(" "));
    let opt = Args::parse();

    let mut cache_dir = env::temp_dir();
    cache_dir.push("rsvg-convert-cache");

    if !cache_dir.exists() {
        fs::create_dir(&cache_dir).context("failed to create temporary directory")?;
    }

    let (output_path, exists) = match hash_input(&opt) {
        Some(hash) => {
            let path = cache_dir.join(hash).with_extension(&opt.format);
            let exists = path.exists();
            (path, exists)
        }
        None => (cache_dir.join("from_stdin").with_extension(&opt.format), false),
    };

    if !exists {
        let mut cmd = Command::new("inkscape");
        match opt.input {
            Some(input) => {
                eprintln!("Reading input from: {}", input.display());
                cmd.arg(input);
            }
            None => {
                eprintln!("Reading input from STDIN");
                cmd.arg("--pipe");
                cmd.stdin(std::process::Stdio::inherit());
            }
        }
        match opt.format.as_str() {
            "png" => cmd.arg("--export-type=png"),
            "pdf" => cmd.arg("--export-type=pdf"),
            "ps" => cmd.arg("--export-type=ps"),
            "eps" => cmd.arg("--export-type=eps"),
            "wmf" => cmd.arg("--export-type=wmf"),
            "emf" => cmd.arg("--export-type=emf"),
            x => return Err(anyhow::anyhow!("Unsupported file format: {}", x)),
        };
        cmd.arg(&format!("--export-filename={}", output_path.display()));

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
        eprintln!("Loading from cache: {}", output_path.display());
    }

    if let Some(output) = opt.output {
        fs::copy(output_path, output).context("Failed to copy output to destination")?;
    }
    else {
        let mut data = std::fs::File::open(&output_path)
            .with_context(|| format!("No output was generated: {}", output_path.display()))?;
        std::io::copy(&mut data, &mut std::io::stdout().lock())
            .context("Failed to write output to stdout")?;
    }

    Ok(())
}
