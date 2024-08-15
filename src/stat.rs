use std::ops::{Add, AddAssign, Sub, SubAssign};

fn naive_sum<const BLOCK: usize, T>(it: &mut impl Iterator<Item = T>) -> (T, bool)
where
    T: Default + AddAssign<T>,
{
    let mut sum = T::default();
    for _ in 0..BLOCK {
        let nxt = it.next();
        if nxt.is_none() {
            return (sum, true);
        }
        sum += nxt.unwrap();
    }
    (sum, false)
}

fn block_kahan_sum<T>(mut it: impl Iterator<Item = T>) -> T
where
    T: Copy + Default + Add<T, Output = T> + Sub<T, Output = T> + AddAssign<T> + SubAssign<T>,
{
    let mut sum = T::default();
    // Correction.
    let mut c = T::default();

    loop {
        let (x, is_over) = naive_sum::<256, T>(&mut it);
        let y = x - c;
        let t = sum + y;
        c = (t - sum) - y;
        sum = t;
        if is_over {
            break;
        }
    }
    sum
}

fn get_mean(slice: &[f64]) -> Option<f64> {
    if slice.len() == 0 {
        None
    } else {
        Some(block_kahan_sum(slice.into_iter().cloned()) / (slice.len() as f64))
    }
}

fn get_variance(slice: &[f64], mean: f64) -> f64 {
    // If the slice has a mean then the length > 0.
    block_kahan_sum(
        slice
            .into_iter()
            .cloned()
            .map(|elem| (elem - mean) * (elem - mean)),
    ) / (slice.len() as f64)
}

#[derive(Debug)]
pub(super) struct Stats {
    pub(super) mean: f64,
    pub(super) variance: f64,
}

impl Stats {
    pub fn try_from_slice(slice: &[f64]) -> Option<Self> {
        let mean = get_mean(slice)?;
        let variance = get_variance(slice, mean);
        Some(Self {
            mean: mean,
            variance: variance,
        })
    }
}

mod tests {
    use super::Stats;

    #[test]
    fn test_stat_some() {
        let ar = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let Stats { mean, variance } = Stats::try_from_slice(&ar).unwrap();

        assert!((mean - 3.0).abs() < 1e-9);
        assert!((variance - 2.0).abs() < 1e-9);
    }

    #[test]
    fn test_stat_none() {
        let stats = Stats::try_from_slice(&[]);
        assert!(stats.is_none());
    }
}
