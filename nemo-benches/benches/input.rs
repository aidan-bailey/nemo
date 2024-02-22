use criterion::{criterion_group, criterion_main, BatchSize, Criterion};
use nemo::model::{Constant, Identifier};
use nemo_physical::{datavalues::AnyDataValue, dictionary::HashMapDictionary};
use rand::{distributions::Alphanumeric, prelude::*};
use rand_pcg::Pcg64;

pub fn benchmark_input(c: &mut Criterion) {
    let mut rng = Pcg64::seed_from_u64(21564);

    let strings = (0..10000)
        .map(|_| {
            (&mut rng)
                .sample_iter(Alphanumeric)
                .take(20)
                .map(char::from)
                .collect::<String>()
        })
        .collect::<Vec<_>>();
    let terms = strings
        .iter()
        .map(|s| AnyDataValue::new_iri(format!("http://example.org/{s}")))
        .collect::<Vec<_>>();
    let iris = terms.iter().map(|t| t.to_string()).collect::<Vec<_>>();

    let mut group = c.benchmark_group("input");

    group.bench_function("read_strings", |b| {
        b.iter_batched(
            || {
                let dict = std::cell::RefCell::new(HashMapDictionary::default());
                (strings.clone(), dict)
            },
            |(input, dict)| {
                let mut pcbp =
                    PhysicalBuilderProxyEnum::String(PhysicalStringColumnBuilderProxy::new(&dict));
                let mut lcbp = LogicalStringColumnBuilderProxy::new(&mut pcbp);
                for str in input {
                    lcbp.add(str).unwrap();
                }
            },
            BatchSize::SmallInput,
        )
    });

    group.bench_function("read_terms", |b| {
        b.iter_batched(
            || {
                let dict = std::cell::RefCell::new(HashMapDictionary::default());
                (terms.clone(), dict)
            },
            |(input, dict)| {
                let mut pcbp =
                    PhysicalBuilderProxyEnum::String(PhysicalStringColumnBuilderProxy::new(&dict));
                let mut lcbp = LogicalAnyColumnBuilderProxy::new(&mut pcbp);
                for term in input {
                    lcbp.add(term).unwrap();
                }
            },
            BatchSize::SmallInput,
        )
    });

    group.bench_function("read_iris", |b| {
        b.iter_batched(
            || {
                let dict = std::cell::RefCell::new(HashMapDictionary::default());
                (iris.clone(), dict)
            },
            |(input, dict)| {
                let mut pcbp =
                    PhysicalBuilderProxyEnum::String(PhysicalStringColumnBuilderProxy::new(&dict));
                let mut lcbp =
                    LogicalAnyColumnBuilderProxy::new(&mut pcbp).into_parser::<Constant>();
                for iri in input {
                    lcbp.add(iri).unwrap();
                }
            },
            BatchSize::SmallInput,
        )
    });

    group.finish();
}

criterion_group!(benches, benchmark_input);
criterion_main!(benches);
