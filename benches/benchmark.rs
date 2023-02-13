use criterion::{black_box, criterion_group, criterion_main, Criterion};
use granular_plugin::load_wav;

pub fn wav_file_load_bm(c: &mut Criterion) {
    c.bench_function("WAV file loading", |b| {
        b.iter(|| load_wav(black_box("tests/amen_br.wav")))
    });
}

criterion_group!(benches, wav_file_load_bm);
criterion_main!(benches);
