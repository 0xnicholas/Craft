use std::path::PathBuf;
use std::process::ExitCode;

fn usage() -> ! {
    eprintln!("usage: craft-assets <pixel-map.json> [out-dir]");
    eprintln!("       craft-assets font [out-dir]");
    eprintln!("       craft-assets wav [out-dir]");
    std::process::exit(2);
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.is_empty() {
        usage();
    }
    match args[0].as_str() {
        "font" => run_font(&args[1..]),
        "wav" => run_wav(&args[1..]),
        _ => run_pixel_map(&args),
    }
}

fn out_dir_for(args: &[String], fallback: PathBuf) -> PathBuf {
    match args.first() {
        Some(dir) => PathBuf::from(dir),
        None => fallback,
    }
}

fn run_pixel_map(args: &[String]) -> ExitCode {
    if args.len() > 2 {
        usage();
    }
    let input = PathBuf::from(&args[0]);
    let out_dir = out_dir_for(
        &args[1..],
        input.parent().map(PathBuf::from).unwrap_or_default(),
    );
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

fn run_font(args: &[String]) -> ExitCode {
    if args.len() > 1 {
        usage();
    }
    let out_dir = out_dir_for(args, PathBuf::from("."));
    match craft_assets::builtin_font() {
        Ok(png) => {
            let png_path = out_dir.join("font.png");
            let meta_path = out_dir.join("font.json");
            let write = std::fs::write(&png_path, png)
                .and_then(|_| std::fs::write(&meta_path, craft_assets::font_metadata()));
            match write {
                Ok(()) => {
                    println!("{}", png_path.display());
                    println!("{}", meta_path.display());
                    ExitCode::SUCCESS
                }
                Err(e) => {
                    eprintln!("craft-assets: {e}");
                    ExitCode::FAILURE
                }
            }
        }
        Err(e) => {
            eprintln!("craft-assets: {e}");
            ExitCode::FAILURE
        }
    }
}

fn run_wav(args: &[String]) -> ExitCode {
    if args.len() > 1 {
        usage();
    }
    let out_dir = out_dir_for(args, PathBuf::from("."));
    let mut ok = true;
    for kind in [
        craft_assets::ToneKind::Jump,
        craft_assets::ToneKind::Coin,
        craft_assets::ToneKind::Win,
    ] {
        match craft_assets::wav_tone(kind) {
            Ok(wav) => {
                let path = out_dir.join(kind.file_name());
                if let Err(e) = std::fs::write(&path, wav) {
                    eprintln!("craft-assets: {e}");
                    ok = false;
                } else {
                    println!("{}", path.display());
                }
            }
            Err(e) => {
                eprintln!("craft-assets: {e}");
                ok = false;
            }
        }
    }
    if ok {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}
