//! topbar: which side of Start each app button is on and in what order; the user drags them, the places are kept.
use std::collections::HashSet;

/// Button ids left and right of Start, both from left to right. Apps the user placed stay listed while closed, so they
/// come back where they were. An id may end in "#..." (several buttons of one app): kept without it.
#[derive(Default, Debug, Clone, PartialEq)]
pub struct Order {
    pub left: Vec<String>,
    pub right: Vec<String>,
    placed: HashSet<String>, // what the user's own arrangement holds
}

fn base(id: &str) -> &str {
    id.split('#').next().unwrap_or(id)
}

impl Order {
    /// From its file: one "L<tab>id" or "R<tab>id" per line.
    pub fn parse(s: &str) -> Order {
        let mut o = Order::default();
        for (side, id) in s.lines().filter_map(|l| l.split_once('\t')) {
            if !id.is_empty() && o.placed.insert(id.to_string()) {
                if side == "L" { o.left.push(id.into()) } else { o.right.push(id.into()) }
            }
        }
        o
    }

    pub fn text(&self) -> String {
        let mut seen = HashSet::new();
        let lines = self.left.iter().map(|i| ("L", i)).chain(self.right.iter().map(|i| ("R", i)));
        lines.filter(|(_, i)| self.placed.contains(base(i)) && seen.insert(base(i))).map(|(s, i)| format!("{s}\t{}\n", base(i))).collect()
    }

    fn has(&self, id: &str) -> bool {
        self.left.iter().chain(&self.right).any(|x| x == id)
    }

    /// The buttons there are now: new ones get a place, closed ones the user never placed lose theirs. Returns the ids
    /// left and right of Start.
    pub fn update(&mut self, now: &[String]) -> (Vec<String>, Vec<String>) {
        let keep = |x: &String| now.contains(x) || self.placed.contains(x);
        self.left.retain(keep);
        self.right.retain(keep);
        for id in now {
            if self.has(id) {
                continue;
            }
            let b = base(id);
            let (l, r) = (self.left.iter().position(|x| base(x) == b), self.right.iter().position(|x| base(x) == b));
            match (l, r) {
                (Some(i), _) => self.left.insert(i + 1, id.clone()), // another button of an app that has its place
                (_, Some(i)) => self.right.insert(i + 1, id.clone()),
                _ => {
                    // the shorter side, at its far end
                    let count = |v: &Vec<String>| v.iter().filter(|x| now.contains(x)).count();
                    if count(&self.right) <= count(&self.left) { self.right.push(id.clone()) } else { self.left.insert(0, id.clone()) }
                }
            }
        }
        let seen = |v: &Vec<String>| v.iter().filter(|x| now.contains(x)).cloned().collect();
        (seen(&self.left), seen(&self.right))
    }

    /// Puts `id` at place `at` among the visible buttons of a side (`vis`, without it).
    pub fn put(&mut self, id: &str, left: bool, at: usize, vis: &[String]) {
        self.left.retain(|x| x != id);
        self.right.retain(|x| x != id);
        let v = if left { &mut self.left } else { &mut self.right };
        let i = vis.get(at).and_then(|n| v.iter().position(|x| x == n)).or_else(|| vis.last().and_then(|p| v.iter().position(|x| x == p)).map(|i| i + 1)).unwrap_or(v.len());
        v.insert(i, id.to_string());
    }

    /// The arrangement as it stands is the user's own from now on.
    pub fn settle(&mut self, now: &[String]) {
        self.placed.extend(now.iter().map(|x| base(x).to_string()));
        while self.placed.len() > 300 {
            let Some(old) = self.placed.iter().find(|x| !now.iter().any(|n| base(n) == x.as_str())).cloned() else { break };
            self.placed.remove(&old);
        }
    }

    /// A button known by another id from now on (its app was pinned, unpinned, or named itself late).
    pub fn rename(&mut self, old: &str, new: &str) {
        if old == new || !self.has(old) {
            return;
        }
        let was = self.placed.contains(base(old));
        self.left.retain(|x| x != new);
        self.right.retain(|x| x != new);
        if let Some(x) = self.left.iter_mut().chain(self.right.iter_mut()).find(|x| *x == old) {
            *x = new.to_string();
        }
        if was {
            self.placed.insert(base(new).to_string());
        }
    }
}

/// Where a button dragged to `center` (px from the middle of Start) goes: its side and place among the side's `n`
/// others, counted from the left. `pitch` is the distance between buttons.
pub fn drop_at(center: i32, pitch: i32, nleft: usize, nright: usize) -> (bool, usize) {
    let left = center < 0;
    let n = if left { nleft } else { nright };
    let k = ((center.abs() + pitch / 2) / pitch.max(1)).clamp(1, n as i32 + 1) as usize; // the k-th from Start
    (left, if left { n + 1 - k } else { k - 1 })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn v(s: &str) -> Vec<String> {
        s.split_whitespace().map(String::from).collect()
    }

    #[test]
    fn apps_spread_to_both_sides_of_start() {
        let mut o = Order::default();
        assert_eq!(o.update(&v("a b c d e")), (v("d b"), v("a c e")));
        // one closes: nobody else changes sides
        assert_eq!(o.update(&v("b c d e")), (v("d b"), v("c e")));
        // a new one balances them
        assert_eq!(o.update(&v("b c d e f")), (v("d b"), v("c e f")));
        assert_eq!(o.update(&v("b c d e f g")), (v("g d b"), v("c e f")));
    }

    #[test]
    fn places_survive_closing_and_a_restart() {
        let mut o = Order::default();
        let now = v("a b c d");
        let (l, r) = o.update(&now);
        assert_eq!((l, r.clone()), (v("d b"), v("a c")));
        o.put("c", true, 1, &v("d b")); // between d and b
        o.settle(&now);
        assert_eq!(o.update(&now), (v("d c b"), v("a")));
        assert_eq!(o.update(&v("a b d")), (v("d b"), v("a")));
        assert_eq!(o.update(&now), (v("d c b"), v("a")));
        let mut again = Order::parse(&o.text());
        assert_eq!(again.update(&v("c")), (v("c"), v("")));
        assert_eq!(again.update(&now), (v("d c b"), v("a")));
        // an app opened now and then takes no place for good
        again.update(&v("a b c d x"));
        assert!(!again.text().contains('x'));
    }

    #[test]
    fn every_drop_lands_where_it_was_let_go() {
        let now = v("a b c d e f g");
        let pitch = 60;
        for id in &now {
            for slot in (-5i32..=5).filter(|s| *s != 0) {
                let mut o = Order::default();
                let (l, r) = o.update(&now);
                let rest = |x: &Vec<String>| x.iter().filter(|y| *y != id).cloned().collect::<Vec<_>>();
                let (l, r) = (rest(&l), rest(&r));
                let (left, at) = drop_at(slot * pitch, pitch, l.len(), r.len());
                o.put(id, left, at, if left { &l } else { &r });
                let (l2, r2) = o.update(&now);
                assert_eq!(l2.len() + r2.len(), now.len());
                let got = if left { -((l2.len() - l2.iter().position(|x| x == id).unwrap()) as i32) } else { r2.iter().position(|x| x == id).unwrap() as i32 + 1 };
                let far = if left { -(l.len() as i32 + 1) } else { r.len() as i32 + 1 };
                assert_eq!(got, if left { slot.max(far) } else { slot.min(far) }, "{id} to {slot}");
                assert_eq!((rest(&l2), rest(&r2)), (l, r), "the others keep their order");
            }
        }
    }

    #[test]
    fn several_buttons_of_one_app_and_new_names() {
        let mut o = Order::parse("L\tjava\nR\ta\n");
        assert_eq!(o.update(&v("java#1 java#2 a")), (v("java#2 java#1"), v("a")));
        o.settle(&v("java#1 java#2 a"));
        assert_eq!(o.text(), "L\tjava\nR\ta\n");
        o.rename("a", "a|id");
        assert_eq!(o.update(&v("java#1 a|id")), (v("java#1"), v("a|id")));
        assert!(Order::parse("garbage\n\nL\t\nL\tx\nR\tx\n").update(&v("x")) == (v("x"), v("")));
    }
}
