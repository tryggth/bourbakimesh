//! Criterion benchmarks for bourbaki-kernel StrategyExtractor and CICDecompiler.

use bourbaki_ir::{LogicalPayload, Move, Polarity, StrategyNode, StrategyTree};
use bourbaki_kernel::{CICDecompiler, StrategyExtractor, Term};
use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};

fn generate_strategy_tree(pairs: usize) -> StrategyTree {
    let root = Move::root_goal("A -> A");
    let mut tree = StrategyTree::from_root(root);

    let mut current_node = tree.root.as_mut().unwrap();
    let mut step = 1;

    for i in 0..pairs {
        // Opponent question
        let opp_node = StrategyNode::new(Move::question(
            step,
            Polarity::Opponent,
            step - 1,
            LogicalPayload::InstantiateUniversal {
                term_repr: format!("x_{} : Prop", i),
            },
        ));
        step += 1;
        current_node.add_child(opp_node);
        current_node = current_node.children.last_mut().unwrap();

        // Proponent answer (leaf at the final step)
        if i == pairs - 1 {
            let prop_node = StrategyNode::new(Move::answer(
                step,
                Polarity::Proponent,
                step - 1,
                LogicalPayload::ProvideWitness {
                    term_repr: format!("x_{}", i),
                },
            ));
            current_node.add_child(prop_node);
        }
    }

    tree
}

fn bench_strategy_extraction(c: &mut Criterion) {
    let mut group = c.benchmark_group("strategy_extraction");
    for pairs in [2, 5, 15].iter() {
        let tree = generate_strategy_tree(*pairs);
        group.bench_with_input(BenchmarkId::from_parameter(pairs), pairs, |b, _| {
            b.iter(|| {
                let term = StrategyExtractor::compile_strategy(black_box(&tree)).unwrap();
                black_box(term);
            });
        });
    }
    group.finish();
}

fn bench_term_decompilation(c: &mut Criterion) {
    let prop_type = Term::arrow(Term::var("A"), Term::var("A"));
    let proof_term = Term::lam("h", Term::var("A"), Term::var("h"));

    c.bench_function("term_decompilation_identity", |b| {
        b.iter(|| {
            let tree = CICDecompiler::term_to_strategy(
                black_box("identity"),
                black_box(&prop_type),
                black_box(&proof_term),
            )
            .unwrap();
            black_box(tree);
        });
    });
}

criterion_group!(benches, bench_strategy_extraction, bench_term_decompilation);
criterion_main!(benches);
