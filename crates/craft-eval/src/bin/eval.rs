use std::collections::BTreeMap;
use std::path::PathBuf;
use std::process::ExitCode;

use craft_eval::{Backend, StubBackend, run_benchmark, summary};
use craft_kernel::NodeRegistry;

struct CliArgs {
    benchmark: Option<PathBuf>,
    dir: Option<PathBuf>,
    backend_name: String,
}

fn parse_args() -> Result<CliArgs, String> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut cli = CliArgs {
        benchmark: None,
        dir: None,
        backend_name: "stub".to_string(),
    };
    let mut i = 0;
    while i < args.len() {
        let a = &args[i];
        match a.as_str() {
            "--benchmark" => {
                i += 1;
                let v = args.get(i).ok_or("missing value for --benchmark")?.clone();
                cli.benchmark = Some(PathBuf::from(v));
            }
            "--dir" => {
                i += 1;
                let v = args.get(i).ok_or("missing value for --dir")?.clone();
                cli.dir = Some(PathBuf::from(v));
            }
            "--backend" => {
                i += 1;
                let v = args.get(i).ok_or("missing value for --backend")?.clone();
                cli.backend_name = v;
            }
            "--help" | "-h" => {
                print_help();
                std::process::exit(0);
            }
            other => return Err(format!("unknown arg: {other}")),
        }
        i += 1;
    }
    Ok(cli)
}

fn print_help() {
    println!("craft-eval — agent benchmark runner");
    println!("  --benchmark <file.json>   run one benchmark");
    println!("  --dir <dir>                run all benchmarks/*.json in dir");
    println!("  --backend <name>           stub (default)");
    println!("  --help                     this help");
}

fn backend_from_name(name: &str) -> Box<dyn Backend> {
    match name {
        "stub" => Box::new(StubBackend::deterministic(BTreeMap::new())),
        _ => panic!("unknown backend {name}; only `stub` is compiled in this build"),
    }
}

fn main() -> ExitCode {
    let cli = match parse_args() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("error: {e}");
            print_help();
            return ExitCode::from(2);
        }
    };

    let backend = backend_from_name(&cli.backend_name);
    let mut registry = NodeRegistry::new();
    registry.instantiate_all();

    if let Some(path) = cli.benchmark {
        let spec = match craft_eval::load_benchmark(&path) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("failed to load {}: {e}", path.display());
                return ExitCode::from(1);
            }
        };
        let report = run_benchmark(&spec, &registry, &*backend);
        println!(
            "{}: {} (hash {} vs {}, components {}, signals {})",
            report.name,
            if report.passed { "PASS" } else { "FAIL" },
            report.final_hash,
            report.expected_hash,
            if report.components_failing.is_empty() {
                "ok"
            } else {
                "failing"
            },
            if report.missing_signals.is_empty() && report.unexpected_signals.is_empty() {
                "ok"
            } else {
                "failing"
            },
        );
        return if report.passed {
            ExitCode::SUCCESS
        } else {
            ExitCode::from(1)
        };
    }

    if let Some(dir) = cli.dir {
        let mut specs = Vec::new();
        let read = std::fs::read_dir(&dir);
        match read {
            Ok(it) => {
                for entry in it.flatten() {
                    let p = entry.path();
                    if p.extension().and_then(|e| e.to_str()) == Some("json") {
                        match craft_eval::load_benchmark(&p) {
                            Ok(spec) => specs.push(spec),
                            Err(e) => eprintln!("skip {}: {e}", p.display()),
                        }
                    }
                }
            }
            Err(e) => {
                eprintln!("failed to read dir {}: {e}", dir.display());
                return ExitCode::from(1);
            }
        }
        specs.sort_by(|a, b| a.name.cmp(&b.name));

        let mut all_ok = true;
        for spec in &specs {
            let report = run_benchmark(spec, &registry, &*backend);
            let status = if report.passed { "PASS" } else { "FAIL" };
            println!(
                "{}: {} ({}, {}ms)",
                report.name,
                status,
                report.ticks_run,
                report.duration_micros / 1000,
            );
            if !report.passed {
                all_ok = false;
                if !report.components_failing.is_empty() {
                    println!("    components_failing: {:?}", report.components_failing);
                }
                if !report.missing_signals.is_empty() {
                    println!("    missing_signals: {:?}", report.missing_signals);
                }
                if !report.unexpected_signals.is_empty() {
                    println!("    unexpected_signals: {:?}", report.unexpected_signals);
                }
            }
        }
        let (passed, total) = summary(
            &specs
                .iter()
                .map(|s| run_benchmark(s, &registry, &*backend))
                .collect::<Vec<_>>(),
        );
        println!("\n{total} benchmarks, {passed} passed");
        return if all_ok {
            ExitCode::SUCCESS
        } else {
            ExitCode::from(1)
        };
    }

    eprintln!("nothing to do; specify --benchmark <file> or --dir <dir>");
    print_help();
    ExitCode::from(2)
}
