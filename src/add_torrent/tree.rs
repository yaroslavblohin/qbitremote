use super::File;
use std::collections::{HashMap, HashSet};

pub struct Node {
    pub name: String,
    pub path: String,
    pub depth: usize,
    pub file: Option<usize>,
    pub size: u64,
    pub count: usize,
    parent: usize,
    children: Vec<usize>,
}
#[derive(Default)]
pub struct Tree {
    pub nodes: Vec<Node>,
}
#[derive(Clone, Copy, Default)]
pub struct Stats {
    pub selected: usize,
    pub priority: Option<u8>,
}
impl Tree {
    pub fn new(files: &[File]) -> Self {
        let mut nodes = vec![Node {
            name: String::new(),
            path: String::new(),
            depth: 0,
            file: None,
            size: 0,
            count: 0,
            parent: 0,
            children: vec![],
        }];
        let mut folders = HashMap::<String, usize>::new();
        for (index, file) in files.iter().enumerate() {
            let path = file.path.replace('\\', "/");
            let parts: Vec<_> = path.split('/').filter(|p| !p.is_empty()).collect();
            let mut parent = 0;
            let mut prefix = String::new();
            for (depth, part) in parts.iter().enumerate() {
                if !prefix.is_empty() {
                    prefix.push('/');
                }
                prefix.push_str(part);
                let leaf = depth + 1 == parts.len();
                let id = if !leaf {
                    folders.get(&prefix).copied()
                } else {
                    None
                };
                let id = id.unwrap_or_else(|| {
                    let id = nodes.len();
                    nodes.push(Node {
                        name: (*part).into(),
                        path: prefix.clone(),
                        depth,
                        file: leaf.then_some(index),
                        size: if leaf { file.size } else { 0 },
                        count: usize::from(leaf),
                        parent,
                        children: vec![],
                    });
                    nodes[parent].children.push(id);
                    if !leaf {
                        folders.insert(prefix.clone(), id);
                    }
                    id
                });
                parent = id;
            }
        }
        for id in (1..nodes.len()).rev() {
            let parent = nodes[id].parent;
            nodes[parent].size = nodes[parent].size.saturating_add(nodes[id].size);
            nodes[parent].count += nodes[id].count;
        }
        for id in 0..nodes.len() {
            let mut children = std::mem::take(&mut nodes[id].children);
            children.sort_by_cached_key(|child| {
                (
                    nodes[*child].file.is_some(),
                    nodes[*child].name.to_lowercase(),
                )
            });
            nodes[id].children = children;
        }
        Self { nodes }
    }
    pub fn default_collapsed(&self) -> HashSet<usize> {
        self.nodes
            .iter()
            .enumerate()
            .skip(1)
            .filter(|(_, n)| n.file.is_none() && n.depth >= 2)
            .map(|(id, _)| id)
            .collect()
    }
    pub fn folders(&self) -> HashSet<usize> {
        self.nodes
            .iter()
            .enumerate()
            .skip(1)
            .filter(|(_, n)| n.file.is_none())
            .map(|(id, _)| id)
            .collect()
    }
    pub fn visible(&self, collapsed: &HashSet<usize>, query: &str) -> Vec<usize> {
        if self.nodes.is_empty() {
            return vec![];
        }
        let query = query.trim().to_lowercase();
        let searching = !query.is_empty();
        let mut keep = vec![!searching; self.nodes.len()];
        if searching {
            for (id, node) in self.nodes.iter().enumerate().skip(1) {
                if node.file.is_some() && node.path.to_lowercase().contains(&query) {
                    let mut id = id;
                    while id != 0 && !keep[id] {
                        keep[id] = true;
                        id = self.nodes[id].parent;
                    }
                }
            }
        }
        let mut out = Vec::new();
        let mut stack: Vec<_> = self.nodes[0].children.iter().rev().copied().collect();
        while let Some(id) = stack.pop() {
            if !keep[id] {
                continue;
            }
            out.push(id);
            if searching || !collapsed.contains(&id) {
                stack.extend(self.nodes[id].children.iter().rev().copied());
            }
        }
        out
    }
    pub fn stats(&self, files: &[File]) -> Vec<Stats> {
        let mut stats = vec![Stats::default(); self.nodes.len()];
        for (id, node) in self.nodes.iter().enumerate().rev() {
            if let Some(index) = node.file {
                stats[id] = Stats {
                    selected: usize::from(files[index].priority > 0),
                    priority: Some(files[index].priority),
                };
            } else {
                stats[id].selected = node.children.iter().map(|c| stats[*c].selected).sum();
                stats[id].priority = node.children.first().and_then(|first| {
                    let priority = stats[*first].priority;
                    node.children
                        .iter()
                        .all(|c| stats[*c].priority == priority)
                        .then_some(priority)
                        .flatten()
                });
            }
        }
        stats
    }
    fn indices(&self, id: usize) -> Vec<usize> {
        let mut stack = vec![id];
        let mut out = vec![];
        while let Some(id) = stack.pop() {
            if let Some(file) = self.nodes[id].file {
                out.push(file);
            } else {
                stack.extend(self.nodes[id].children.iter().copied());
            }
        }
        out
    }
    pub fn select(&self, id: usize, files: &mut [File], selected: bool) {
        for index in self.indices(id) {
            files[index].priority = if selected {
                files[index].priority.max(1)
            } else {
                0
            };
        }
    }
    pub fn priority(&self, id: usize, files: &mut [File], priority: u8) {
        for index in self.indices(id) {
            files[index].priority = priority;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn filtered_folder_priority_includes_hidden_children_but_not_similarly_named_siblings() {
        let mut files = vec![
            File {
                path: "Game/Pack/a.bin".into(),
                size: 10,
                priority: 6,
            },
            File {
                path: "Game/Pack/b.bin".into(),
                size: 20,
                priority: 0,
            },
            File {
                path: "Game/Package/c.bin".into(),
                size: 30,
                priority: 1,
            },
            File {
                path: "Game\\Pack\\Я.txt".into(),
                size: 40,
                priority: 1,
            },
        ];
        let tree = Tree::new(&files);
        let pack = tree
            .nodes
            .iter()
            .position(|n| n.path == "Game/Pack")
            .unwrap();
        let collapsed = tree.folders();
        assert_eq!(tree.visible(&collapsed, "a.bin").len(), 3);
        tree.priority(pack, &mut files, 7);
        assert_eq!(
            files.iter().map(|f| f.priority).collect::<Vec<_>>(),
            [7, 7, 1, 7]
        );
        tree.select(pack, &mut files, false);
        let stats = tree.stats(&files);
        assert_eq!(stats[pack].selected, 0);
        assert_eq!(stats[pack].priority, Some(0));
        assert_eq!(files[2].priority, 1);
    }
    #[test]
    fn five_thousand_files_keep_api_identity_through_sort_collapse_and_search() {
        let files = (0..5000)
            .rev()
            .map(|i| File {
                path: format!("Root/Folder-{:03}/file-{i:05}.bin", i % 100),
                size: 100,
                priority: if i % 2 == 0 { 6 } else { 0 },
            })
            .collect::<Vec<_>>();
        let tree = Tree::new(&files);
        let collapsed = tree.folders();
        assert_eq!(tree.visible(&collapsed, "").len(), 1);
        let found = tree.visible(&collapsed, "file-00000.bin");
        assert_eq!(found.len(), 3);
        assert_eq!(tree.nodes[*found.last().unwrap()].file, Some(4999));
        let root = tree.visible(&collapsed, "")[0];
        assert_eq!(tree.nodes[root].count, 5000);
        assert_eq!(tree.stats(&files)[root].selected, 2500);
        assert_eq!(tree.nodes[root].size, 500000);
    }
    fn files() -> Vec<File> {
        vec![
            File {
                path: "Game/Z/bin.exe".into(),
                size: 10,
                priority: 6,
            },
            File {
                path: "Game/A/a.txt".into(),
                size: 20,
                priority: 0,
            },
            File {
                path: "readme.txt".into(),
                size: 5,
                priority: 1,
            },
        ]
    }
    #[test]
    fn folder_selection_changes_descendants_and_preserves_original_indices() {
        let mut files = files();
        let tree = Tree::new(&files);
        let game = tree.nodes.iter().position(|n| n.path == "Game").unwrap();
        let stats = tree.stats(&files);
        assert_eq!(tree.nodes[game].size, 30);
        assert_eq!(stats[game].selected, 1);
        assert_eq!(stats[game].priority, None);
        tree.select(game, &mut files, true);
        assert_eq!(
            files.iter().map(|f| f.priority).collect::<Vec<_>>(),
            [6, 1, 1]
        );
        tree.priority(game, &mut files, 7);
        assert_eq!(
            files.iter().map(|f| f.priority).collect::<Vec<_>>(),
            [7, 7, 1]
        );
        tree.select(game, &mut files, false);
        assert_eq!(
            files.iter().map(|f| f.priority).collect::<Vec<_>>(),
            [0, 0, 1]
        );
    }
    #[test]
    fn collapse_and_search_only_change_visibility() {
        let files = files();
        let tree = Tree::new(&files);
        let all = tree.visible(&HashSet::new(), "");
        let game = all[0];
        assert_eq!(tree.nodes[game].name, "Game");
        let collapsed = HashSet::from([game]);
        assert_eq!(tree.visible(&collapsed, "").len(), 2);
        let found = tree.visible(&collapsed, "BIN.EXE");
        assert_eq!(found.len(), 3);
        assert_eq!(tree.nodes[*found.last().unwrap()].file, Some(0));
        assert_eq!(tree.visible(&collapsed, "not found").len(), 0);
    }
}
