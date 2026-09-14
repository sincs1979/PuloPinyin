pub const PAGE_SIZE: usize = 10;

pub fn page_count(len: usize) -> usize {
    if len == 0 {
        0
    } else {
        (len + PAGE_SIZE - 1) / PAGE_SIZE
    }
}

pub fn page_slice<T>(items: &[T], page: usize) -> &[T] {
    let start = page.saturating_mul(PAGE_SIZE);
    if start >= items.len() {
        return &[];
    }
    let end = (start + PAGE_SIZE).min(items.len());
    &items[start..end]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pages_are_stable_windows() {
        let items: Vec<i32> = (0..25).collect();
        assert_eq!(page_slice(&items, 0), &[0, 1, 2, 3, 4, 5, 6, 7, 8, 9]);
        assert_eq!(
            page_slice(&items, 1),
            &[10, 11, 12, 13, 14, 15, 16, 17, 18, 19]
        );
        assert_eq!(page_slice(&items, 2), &[20, 21, 22, 23, 24]);
        assert_eq!(page_count(25), 3);
    }
}
