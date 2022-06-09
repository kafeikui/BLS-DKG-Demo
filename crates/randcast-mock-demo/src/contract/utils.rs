use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

pub fn calculate_hash<T: Hash>(t: &T) -> u64 {
    let mut s = DefaultHasher::new();
    t.hash(&mut s);
    s.finish()
}

pub fn choose_randomly_from_indices(
    seed: usize,
    indices: &[usize],
    mut count: usize,
) -> Vec<usize> {
    let mut vec = indices.to_vec();

    let mut res: Vec<usize> = Vec::new();

    let mut hash = seed;

    while count > 0 && !vec.is_empty() {
        hash = calculate_hash(&hash) as usize;

        let index = map_to_qualified_indices(hash % (vec.len() + 1), &vec);

        res.push(index);

        vec.retain(|&x| x != index);

        count -= 1;
    }

    res
}

pub fn map_to_qualified_indices(mut index: usize, qualified_indices: &[usize]) -> usize {
    let max = qualified_indices.iter().max().unwrap();

    while !qualified_indices.contains(&index) {
        index = (index + 1) % (max + 1);
    }

    index
}
