//! Criterion benchmarks for bourbaki-ir dialogue view extraction and trace validation.

use bourbaki_ir::{LogicalPayload, Move, MoveKind, PlayTrace, Polarity};
use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};

fn generate_alternating_trace(depth: usize) -> PlayTrace {
    let mut trace = PlayTrace::new();
    let mut moves = Vec::with_capacity(depth);
    moves.push(Move::root_goal("A -> A"));

    for i in 1..depth {
        let player = if i % 2 == 1 {
            Polarity::Opponent
        } else {
            Polarity::Proponent
        };
        let kind = if i % 2 == 1 {
            MoveKind::Question
        } else {
            MoveKind::Answer
        };
        let justifier = Some(i - 1);
        let payload = if i % 2 == 1 {
            LogicalPayload::AttackHypothesis { hyp_id: i }
        } else {
            LogicalPayload::AxiomDischarge { premise_id: i - 1 }
        };
        moves.push(Move::new(i, player, kind, justifier, payload));
    }

    *trace.moves_mut() = moves;
    trace
}

fn bench_p_view_extraction(c: &mut Criterion) {
    let mut group = c.benchmark_group("p_view_extraction");
    for depth in [10, 50, 200].iter() {
        let trace = generate_alternating_trace(*depth);
        group.bench_with_input(BenchmarkId::from_parameter(depth), depth, |b, _| {
            b.iter(|| {
                let p_view = trace.get_p_view();
                black_box(p_view);
            });
        });
    }
    group.finish();
}

fn bench_o_view_extraction(c: &mut Criterion) {
    let mut group = c.benchmark_group("o_view_extraction");
    for depth in [10, 50, 200].iter() {
        let trace = generate_alternating_trace(*depth);
        group.bench_with_input(BenchmarkId::from_parameter(depth), depth, |b, _| {
            b.iter(|| {
                let o_view = trace.get_o_view();
                black_box(o_view);
            });
        });
    }
    group.finish();
}

criterion_group!(benches, bench_p_view_extraction, bench_o_view_extraction);
criterion_main!(benches);
