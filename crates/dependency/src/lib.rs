//! Worksheet dependency graph: ordering, cycle detection, incremental
//! invalidation (spec section 9).
use std::collections::{BTreeMap, BTreeSet, VecDeque};

#[derive(Debug, thiserror::Error)]
#[error("circular dependency: {}", .0.join(" -> "))]
pub struct Cycle(pub Vec<String>);

#[derive(Clone, Debug, Default)]
pub struct DependencyGraph {
    deps: BTreeMap<String, BTreeSet<String>>,
    reverse: BTreeMap<String, BTreeSet<String>>,
}

impl DependencyGraph {
    pub fn new() -> Self { Self::default() }

    pub fn clear(&mut self) { self.deps.clear(); self.reverse.clear(); }

    pub fn define(&mut self, name: &str, dependencies: BTreeSet<String>) {
        self.remove(name);
        for d in &dependencies {
            self.reverse.entry(d.clone()).or_default().insert(name.to_string());
        }
        self.deps.insert(name.to_string(), dependencies);
    }

    pub fn remove(&mut self, name: &str) {
        if let Some(old) = self.deps.remove(name) {
            for d in old {
                if let Some(set) = self.reverse.get_mut(&d) { set.remove(name); }
            }
        }
        self.reverse.remove(name);
    }

    pub fn has(&self, name: &str) -> bool { self.deps.contains_key(name) }
    pub fn names(&self) -> Vec<String> { self.deps.keys().cloned().collect() }
    pub fn dependencies(&self, name: &str) -> BTreeSet<String> { self.deps.get(name).cloned().unwrap_or_default() }
    pub fn dependents(&self, name: &str) -> BTreeSet<String> { self.reverse.get(name).cloned().unwrap_or_default() }

    pub fn external_symbols(&self) -> BTreeSet<String> {
        let mut out = BTreeSet::new();
        for deps in self.deps.values() {
            for d in deps {
                if !self.deps.contains_key(d) { out.insert(d.clone()); }
            }
        }
        out
    }

    /// Deterministic topological order of all definitions.
    pub fn order(&self) -> Result<Vec<String>, Cycle> {
        self.order_subset(&self.deps.keys().cloned().collect())
    }

    fn order_subset(&self, subset: &BTreeSet<String>) -> Result<Vec<String>, Cycle> {
        let mut indegree: BTreeMap<String, usize> = BTreeMap::new();
        for name in subset {
            let count = self.deps.get(name)
                .map(|d| d.iter().filter(|x| subset.contains(*x)).count())
                .unwrap_or(0);
            indegree.insert(name.clone(), count);
        }
        let mut queue: VecDeque<String> = indegree.iter()
            .filter(|(_, d)| **d == 0)
            .map(|(n, _)| n.clone())
            .collect();
        let mut out = Vec::new();
        while let Some(name) = queue.pop_front() {
            out.push(name.clone());
            if let Some(dependents) = self.reverse.get(&name) {
                for dep in dependents {
                    if let Some(d) = indegree.get_mut(dep) {
                        if *d > 0 {
                            *d -= 1;
                            if *d == 0 { queue.push_back(dep.clone()); }
                        }
                    }
                }
            }
        }
        if out.len() != subset.len() {
            let remaining: BTreeSet<String> = indegree.iter()
                .filter(|(_, d)| **d > 0)
                .map(|(n, _)| n.clone())
                .collect();
            return Err(Cycle(self.find_cycle(&remaining)));
        }
        Ok(out)
    }

    fn find_cycle(&self, candidates: &BTreeSet<String>) -> Vec<String> {
        fn dfs(
            g: &DependencyGraph,
            node: &str,
            candidates: &BTreeSet<String>,
            visiting: &mut BTreeSet<String>,
            path: &mut Vec<String>,
        ) -> bool {
            if visiting.contains(node) {
                path.push(node.to_string());
                return true;
            }
            if !candidates.contains(node) { return false; }
            visiting.insert(node.to_string());
            path.push(node.to_string());
            if let Some(deps) = g.deps.get(node) {
                for d in deps {
                    if dfs(g, d, candidates, visiting, path) { return true; }
                }
            }
            path.pop();
            visiting.remove(node);
            false
        }
        let mut path = Vec::new();
        let mut visiting = BTreeSet::new();
        for node in candidates {
            if dfs(self, node, candidates, &mut visiting, &mut path) {
                return path;
            }
        }
        candidates.iter().cloned().collect()
    }

    /// Definitions affected by a change, in recalculation order.
    pub fn affected(&self, changed: &[String]) -> Result<Vec<String>, Cycle> {
        let mut set: BTreeSet<String> = BTreeSet::new();
        let mut queue: VecDeque<String> = changed.iter().cloned().collect();
        while let Some(name) = queue.pop_front() {
            if !self.deps.contains_key(&name) { continue; }
            if set.insert(name.clone()) {
                for d in self.dependents(&name) { queue.push_back(d); }
            }
        }
        self.order_subset(&set)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn set(items: &[&str]) -> BTreeSet<String> { items.iter().map(|s| s.to_string()).collect() }

    #[test]
    fn ordering_and_incremental_affected() {
        let mut g = DependencyGraph::new();
        g.define("a", set(&[]));
        g.define("b", set(&[]));
        g.define("c", set(&["a", "b"]));
        g.define("d", set(&["c"]));
        let order = g.order().unwrap();
        assert!(order.iter().position(|x| x == "c").unwrap() < order.iter().position(|x| x == "d").unwrap());
        let affected = g.affected(&["a".to_string()]).unwrap();
        assert_eq!(affected, vec!["a".to_string(), "c".to_string(), "d".to_string()]);
    }

    #[test]
    fn cycle_detection() {
        let mut g = DependencyGraph::new();
        g.define("x", set(&["y"]));
        g.define("y", set(&["x"]));
        assert!(g.order().is_err());
    }
}
