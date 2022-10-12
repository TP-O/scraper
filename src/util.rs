pub fn get_batch_range(total: usize, batches: usize, order: usize) -> Option<(usize, usize)> {
    // Round up batch_size
    let batch_size = if total % batches == 0 {
        total / batches
    } else {
        total / batches + 1
    };

    let start = batch_size * order;

    if start >= total {
        return None;
    }

    let end = start + batch_size;

    if end >= total {
        return Some((start, total));
    }

    Some((start, end))
}
