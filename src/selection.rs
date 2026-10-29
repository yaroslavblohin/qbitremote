use std::collections::BTreeSet;

#[derive(Default)]
pub struct Selection {
    pub hashes: BTreeSet<String>,
    anchor: Option<String>,
}
impl Selection {
    pub fn clear(&mut self) {
        self.hashes.clear();
        self.anchor = None;
    }
    pub fn retain(&mut self, visible: &[String]) {
        self.hashes.retain(|hash| visible.contains(hash));
        if self
            .anchor
            .as_ref()
            .is_some_and(|hash| !visible.contains(hash))
        {
            self.anchor = None;
        }
    }
    pub fn all(&mut self, visible: &[String]) {
        self.hashes = visible.iter().cloned().collect();
        self.anchor = visible.first().cloned();
    }
    pub fn click(
        &mut self,
        hash: &str,
        visible: &[String],
        ctrl: bool,
        shift: bool,
        secondary: bool,
    ) {
        if secondary && self.hashes.contains(hash) {
            return;
        }
        if shift && !secondary {
            if let Some(start) = self
                .anchor
                .as_ref()
                .and_then(|h| visible.iter().position(|v| v == h))
            {
                if let Some(end) = visible.iter().position(|v| v == hash) {
                    if !ctrl {
                        self.hashes.clear();
                    }
                    self.hashes
                        .extend(visible[start.min(end)..=start.max(end)].iter().cloned());
                    return;
                }
            }
        }
        if ctrl && !secondary {
            if !self.hashes.remove(hash) {
                self.hashes.insert(hash.into());
            }
        } else {
            self.hashes.clear();
            self.hashes.insert(hash.into());
        }
        self.anchor = Some(hash.into());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn rows() -> Vec<String> {
        ["a", "b", "c", "d"].map(String::from).to_vec()
    }
    #[test]
    fn ctrl_toggles_and_plain_click_replaces() {
        let mut s = Selection::default();
        let r = rows();
        s.click("a", &r, false, false, false);
        s.click("c", &r, true, false, false);
        assert_eq!(s.hashes.len(), 2);
        s.click("a", &r, true, false, false);
        assert!(!s.hashes.contains("a"));
        s.click("b", &r, false, false, false);
        assert_eq!(s.hashes, BTreeSet::from(["b".into()]));
    }
    #[test]
    fn shift_uses_current_sort_and_ctrl_adds() {
        let mut s = Selection::default();
        let r = rows();
        s.click("b", &r, false, false, false);
        let reversed: Vec<_> = r.iter().rev().cloned().collect();
        s.click("d", &reversed, false, true, false);
        assert_eq!(s.hashes.len(), 3);
        s.click("a", &r, true, true, false);
        assert_eq!(s.hashes.len(), 4);
    }
    #[test]
    fn right_click_preserves_group_or_selects_target() {
        let mut s = Selection::default();
        let r = rows();
        s.click("a", &r, false, false, false);
        s.click("b", &r, true, false, false);
        s.click("a", &r, false, false, true);
        assert_eq!(s.hashes.len(), 2);
        s.click("d", &r, false, false, true);
        assert_eq!(s.hashes, BTreeSet::from(["d".into()]));
    }
    #[test]
    fn filtering_and_deleted_torrents_are_removed() {
        let mut s = Selection::default();
        s.all(&rows());
        s.retain(&["b".into(), "c".into()]);
        assert_eq!(s.hashes.len(), 2);
        s.retain(&[]);
        assert!(s.hashes.is_empty());
        assert!(s.anchor.is_none());
    }
}
