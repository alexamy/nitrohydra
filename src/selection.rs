pub struct Selection {
    items: Vec<usize>,
}

impl Selection {
    pub fn new() -> Self {
        Self { items: Vec::new() }
    }

    pub fn click(&mut self, index: usize, shift: bool) {
        if shift {
            self.items = vec![index, index];
            return;
        }
        if self.is_duplicated() {
            if self.items[0] != index {
                self.items[1] = index;
            }
        } else if self.items.contains(&index) {
            if self.items.len() == 2 {
                self.items.swap(0, 1);
            }
        } else if self.items.len() == 2 {
            self.items.remove(1);
            self.items.push(index);
        } else {
            self.items.push(index);
        }
    }

    pub fn badge(&self, image_index: usize) -> Option<&str> {
        let pos = self.items.iter().position(|&idx| idx == image_index)?;
        if self.is_duplicated() {
            Some("*")
        } else {
            match pos {
                0 => Some("1"),
                1 => Some("2"),
                _ => None,
            }
        }
    }

    pub fn items(&self) -> &[usize] {
        &self.items
    }

    pub fn len(&self) -> usize {
        self.items.len()
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    pub fn clear(&mut self) {
        self.items.clear();
    }

    fn is_duplicated(&self) -> bool {
        self.items.len() == 2 && self.items[0] == self.items[1]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sel(items: &[usize]) -> Selection {
        Selection { items: items.to_vec() }
    }

    // --- click tests ---

    #[test]
    fn empty_click() {
        let mut s = sel(&[]);
        s.click(0, false);
        assert_eq!(s.items(), &[0]);
    }

    #[test]
    fn empty_shift_click() {
        let mut s = sel(&[]);
        s.click(0, true);
        assert_eq!(s.items(), &[0, 0]);
    }

    #[test]
    fn single_click_different() {
        let mut s = sel(&[0]);
        s.click(1, false);
        assert_eq!(s.items(), &[0, 1]);
    }

    #[test]
    fn single_click_same() {
        let mut s = sel(&[0]);
        s.click(0, false);
        assert_eq!(s.items(), &[0]);
    }

    #[test]
    fn single_shift_click_same() {
        let mut s = sel(&[0]);
        s.click(0, true);
        assert_eq!(s.items(), &[0, 0]);
    }

    #[test]
    fn single_shift_click_different() {
        let mut s = sel(&[0]);
        s.click(1, true);
        assert_eq!(s.items(), &[1, 1]);
    }

    #[test]
    fn pair_click_first_swaps() {
        let mut s = sel(&[0, 1]);
        s.click(0, false);
        assert_eq!(s.items(), &[1, 0]);
    }

    #[test]
    fn pair_click_second_swaps() {
        let mut s = sel(&[0, 1]);
        s.click(1, false);
        assert_eq!(s.items(), &[1, 0]);
    }

    #[test]
    fn pair_click_new_replaces_second() {
        let mut s = sel(&[0, 1]);
        s.click(2, false);
        assert_eq!(s.items(), &[0, 2]);
    }

    #[test]
    fn pair_shift_click_new() {
        let mut s = sel(&[0, 1]);
        s.click(2, true);
        assert_eq!(s.items(), &[2, 2]);
    }

    #[test]
    fn duplicated_click_same() {
        let mut s = sel(&[0, 0]);
        s.click(0, false);
        assert_eq!(s.items(), &[0, 0]);
    }

    #[test]
    fn duplicated_click_different() {
        let mut s = sel(&[0, 0]);
        s.click(1, false);
        assert_eq!(s.items(), &[0, 1]);
    }

    #[test]
    fn duplicated_shift_click_different() {
        let mut s = sel(&[0, 0]);
        s.click(1, true);
        assert_eq!(s.items(), &[1, 1]);
    }

    // --- badge tests ---

    #[test]
    fn badge_empty() {
        let s = sel(&[]);
        assert_eq!(s.badge(0), None);
    }

    #[test]
    fn badge_single_selected() {
        let s = sel(&[0]);
        assert_eq!(s.badge(0), Some("1"));
    }

    #[test]
    fn badge_pair_second() {
        let s = sel(&[0, 1]);
        assert_eq!(s.badge(1), Some("2"));
    }

    #[test]
    fn badge_duplicated() {
        let s = sel(&[0, 0]);
        assert_eq!(s.badge(0), Some("*"));
    }

    #[test]
    fn badge_not_selected() {
        let s = sel(&[0, 1]);
        assert_eq!(s.badge(2), None);
    }
}
