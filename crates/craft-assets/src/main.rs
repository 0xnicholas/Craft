use std::path::PathBuf;
use std::process::ExitCode;

fn usage() -> ! {
    eprintln!("usage: craft-assets <pixel-map.json> [out-dir]");
    std::process::exit(2);
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.is_empty() || args.len() > 2 {
        usage();
    }
    let input = PathBuf::from(&args[0]);
    let out_dir = match args.get(1) {
        Some(dir) => PathBuf::from(dir),
        None => input
            .parent()
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from(".")),
    };
    match craft_assets::generate_dir(&input, &out_dir) {
        Ok(files) => {
            println!("{}", files.png_path.display());
            println!("{}", files.atlas_path.display());
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("craft-assets: {e}");
            ExitCode::FAILURE
        }
    }
}
