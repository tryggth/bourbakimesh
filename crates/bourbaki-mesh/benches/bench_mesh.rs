//! Criterion benchmarks for bourbaki-mesh DAG insertion and cryptographic block hashing.

use bourbaki_mesh::{BlockId, ProofBlock, ProofLedger};
use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};

fn bench_block_creation_and_hashing(c: &mut Criterion) {
    let parent = BlockId::from_bytes([0u8; 32]);

    c.bench_function("proof_block_hashing", |b| {
        b.iter(|| {
            let block = ProofBlock::new(
                black_box(vec![parent]),
                black_box("test_thm".to_string()),
                black_box("A -> A".to_string()),
                black_box(None),
                black_box(None),
                black_box(true),
                black_box(1234567890),
            );
            black_box(block);
        });
    });
}

fn bench_ledger_bulk_insertion(c: &mut Criterion) {
    let mut group = c.benchmark_group("ledger_insertion");
    for size in [50, 200].iter() {
        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, &n| {
            b.iter(|| {
                let mut ledger = ProofLedger::new();
                let genesis = ProofBlock::genesis("True");
                let mut parent_id = genesis.id;
                ledger.insert_block(genesis).unwrap();

                for i in 1..n {
                    let block = ProofBlock::new(
                        vec![parent_id],
                        format!("thm_{}", i),
                        "A -> A".to_string(),
                        None,
                        None,
                        true,
                        1234567890 + i as u64,
                    );
                    parent_id = block.id;
                    ledger.insert_block(block).unwrap();
                }

                black_box(&ledger);
            });
        });
    }
    group.finish();
}

criterion_group!(
    benches,
    bench_block_creation_and_hashing,
    bench_ledger_bulk_insertion
);
criterion_main!(benches);
