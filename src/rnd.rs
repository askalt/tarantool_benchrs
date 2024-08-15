use rand::Rng;

/// Generate sequence of count different numbers in range [L..R].
/// O(count) memory.
pub fn generate_diff_sequence(
    rng: &mut rand::rngs::ThreadRng,
    l: usize,
    r: usize,
    count: usize,
) -> Vec<usize> {
    assert!(count <= r - l + 1);
    let mut replaces = std::collections::HashMap::<usize, usize>::new();
    let mut result = vec![];

    for i in 0..count {
        let lbound = l + i;
        let lbound_replace = if let Some(replace) = replaces.get(&lbound) {
            *replace
        } else {
            lbound
        };

        let num = rng.gen_range(lbound..r + 1) as usize;
        if let Some(replace) = replaces.get(&num) {
            result.push(*replace);
        } else {
            result.push(num);
        }
        replaces.insert(num, lbound_replace);
    }

    result
}

mod test {
    use super::generate_diff_sequence;

    #[test]
    fn test_generate_diff_sequence_full_range() {
        let mut rng = rand::thread_rng();

        for l in 0..100 {
            for r in l..100 {
                let mut res = generate_diff_sequence(&mut rng, l, r, r - l + 1);
                res.sort();
                assert_eq!(res, (l..r + 1).collect::<Vec<usize>>());
            }
        }
    }
}
