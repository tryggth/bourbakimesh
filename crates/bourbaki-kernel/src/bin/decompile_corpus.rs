//! CLI tool for batch Mathlib proof term decompilation and binary corpus export.

use std::env;
use std::fs;
use std::path::PathBuf;
use bourbaki_kernel::corpus::CorpusDecompiler;

fn print_usage() {
    println!("Usage: decompile_corpus --input <input.json> --output <output.bin> [--threads <n>]");
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    let mut input_path = PathBuf::from("data/mathlib_raw.json");
    let mut output_path = PathBuf::from("data/mathlib_corpus.bin");
    let mut threads = num_cpus();

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--input" => {
                if i + 1 < args.len() {
                    input_path = PathBuf::from(&args[i + 1]);
                    i += 1;
                }
            }
            "--output" => {
                if i + 1 < args.len() {
                    output_path = PathBuf::from(&args[i + 1]);
                    i += 1;
                }
            }
            "--threads" => {
                if i + 1 < args.len() {
                    threads = args[i + 1].parse().unwrap_or(threads);
                    i += 1;
                }
            }
            "--help" | "-h" => {
                print_usage();
                return Ok(());
            }
            _ => {}
        }
        i += 1;
    }

    println!("===============================================================");
    println!("⚙️  BourbakiMesh Corpus Decompiler (Threads: {})", threads);
    println!("===============================================================");
    println!("Reading raw theorem export: {:?}", input_path);

    let raw_json = fs::read_to_string(&input_path)?;
    let dataset = CorpusDecompiler::decompile_raw_json(&raw_json)?;

    if let Some(parent) = output_path.parent() {
        fs::create_dir_all(parent)?;
    }

    dataset.save_bincode(&output_path)?;

    // Also write JSON summary alongside binary dataset
    let json_meta_path = output_path.with_extension("json");
    let json_str = dataset.to_json_string()?;
    fs::write(&json_meta_path, json_str)?;

    let meta = fs::metadata(&output_path)?;
    println!("✅ Decompiled {} theorems into {} strategy nodes.", dataset.theorems.len(), dataset.total_nodes);
    println!("💾 Binary corpus saved to: {:?} ({} bytes)", output_path, meta.len());
    println!("📄 JSON metadata saved to: {:?}", json_meta_path);

    Ok(())
}

fn num_cpus() -> usize {
    std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(4)
}
