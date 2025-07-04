use jerky::bit_vectors::Build;
use rand::{Rng, SeedableRng};
use rand_chacha::ChaChaRng;

const SEED_BITS: u64 = 113;
const NUM_BITS: usize = 1 << 20;

fn main() {
    show_memories(0.5);
    show_memories(0.1);
    show_memories(0.01);
}

fn gen_random_bits(len: usize, p: f64, seed: u64) -> Vec<bool> {
    let mut rng = ChaChaRng::seed_from_u64(seed);
    (0..len).map(|_| rng.gen_bool(p)).collect()
}

fn show_memories(p: f64) {
    let bits = gen_random_bits(NUM_BITS, p, SEED_BITS);
    println!("[p = {p}]");

    let bytes = {
        let idx = jerky::bit_vectors::Rank9Sel::from_bits(bits.iter().cloned());
        idx.size_in_bytes()
    };
    print_memory("Rank9Sel", bytes);

    let bytes = {
        let idx =
            jerky::bit_vectors::Rank9Sel::build_from_bits(bits.iter().cloned(), false, true, true)
                .unwrap();
        idx.size_in_bytes()
    };
    print_memory("Rank9Sel (with select hints)", bytes);

    let bytes = {
        let idx = jerky::bit_vectors::DArray::from_bits(bits.iter().cloned());
        idx.size_in_bytes()
    };
    print_memory("DArray", bytes);

    let bytes = {
        let idx = jerky::bit_vectors::DArray::from_bits(bits.iter().cloned()).enable_rank();
        idx.size_in_bytes()
    };
    print_memory("DArray (with rank index)", bytes);
}

fn print_memory(name: &str, bytes: usize) {
    println!(
        "{}: {:.3} bits per bit",
        name,
        (bytes * 8) as f64 / NUM_BITS as f64
    );
}
