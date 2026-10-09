//! Measures a full library index of a real `~/.claude/projects` into a throwaway database.
//! Usage: cargo run --release --example index_benchmark -- <projects-dir> <database-file>
//! Reads transcripts only; prints counts and timings, never transcript content.

use std::env;
use std::path::PathBuf;
use std::time::Instant;

use ccm_lib::db::Database;
use ccm_lib::library::indexer::Indexer;
use ccm_lib::library::queries;

fn main() {
    let arguments: Vec<String> = env::args().skip(1).collect();
    let [projects, database_file] = arguments.as_slice() else {
        eprintln!("uso: index_benchmark <projects-dir> <database-file>");
        std::process::exit(2);
    };
    let database_file = PathBuf::from(database_file);
    let indexer = Indexer::new(Database::open(&database_file).expect("database"), PathBuf::from(projects));

    let started = Instant::now();
    let files = indexer.discover();
    let outcome = indexer.index_paths(&files).expect("index");
    let full_pass = started.elapsed();

    let started = Instant::now();
    let second = indexer.index_paths(&files).expect("reindex");
    let idle_pass = started.elapsed();

    let reader = Database::open(&database_file).expect("reader");
    let sessions = queries::list_sessions(&reader.connection()).expect("list");
    let started = Instant::now();
    let hits = queries::search(&reader.connection(), "teste").expect("search");
    let search_time = started.elapsed();
    let database_bytes = std::fs::metadata(&database_file).map(|metadata| metadata.len()).unwrap_or(0);

    println!("arquivos: {} (alterados: {})", files.len(), outcome.files_changed);
    println!("sessões listadas: {}", sessions.len());
    println!("indexação completa: {:.2?}", full_pass);
    println!("segunda passada (nada mudou): {:.2?} (alterados: {})", idle_pass, second.files_changed);
    println!("busca 'teste': {} resultados em {:.2?}", hits.len(), search_time);
    println!("banco: {:.1} MB", database_bytes as f64 / 1_048_576.0);
}
