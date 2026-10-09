use std::{
    fs,
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};

const MAX_NODES: usize = 30_000;
const MAX_DEPTH: usize = 18;

const IGNORED_DIRECTORIES: &[&str] = &[
    ".git", ".hg", ".svn", ".idea", ".next", ".cache", "target",
    "node_modules", ".venv", "venv", "__pycache__", "bin", "obj",
    "dist", "build",
];

#[derive(Clone, Debug)]
pub struct FileNode {
    pub path: PathBuf,
    pub name: String,
    pub is_dir: bool,
    pub children: Vec<FileNode>,
}

#[derive(Clone, Debug)]
pub struct TreeRow {
    pub path: PathBuf,
    pub name: String,
    pub is_dir: bool,
    pub depth: usize,
}

pub struct WorkspaceIndex {
    pub nodes: Vec<FileNode>,
    pub all_files: Vec<PathBuf>,
    pub truncated: bool,
}

pub fn scan_workspace(root: PathBuf, cancelled: Arc<AtomicBool>) -> WorkspaceIndex {
    let mut all_files = Vec::new();
    let mut node_count = 0usize;
    let mut truncated = false;
    let nodes = scan_directory(
        &root,
        0,
        &cancelled,
        &mut all_files,
        &mut node_count,
        &mut truncated,
    );
    all_files.sort();
    WorkspaceIndex { nodes, all_files, truncated }
}

fn scan_directory(
    directory: &Path,
    depth: usize,
    cancelled: &Arc<AtomicBool>,
    all_files: &mut Vec<PathBuf>,
    node_count: &mut usize,
    truncated: &mut bool,
) -> Vec<FileNode> {
    if depth > MAX_DEPTH || *node_count >= MAX_NODES || cancelled.load(Ordering::Relaxed) {
        *truncated |= *node_count >= MAX_NODES;
        return Vec::new();
    }

    let entries = match fs::read_dir(directory) {
        Ok(entries) => entries,
        Err(_) => return Vec::new(),
    };

    // Bound the amount of directory metadata held in memory, even when one
    // folder contains hundreds of thousands of generated files.
    let remaining = MAX_NODES.saturating_sub(*node_count);
    let mut paths = entries
        .filter_map(Result::ok)
        .filter_map(|entry| {
            let path = entry.path();
            let file_type = entry.file_type().ok()?;
            if file_type.is_symlink() {
                return None;
            }
            let name = entry.file_name().to_string_lossy().to_string();
            if file_type.is_dir() && IGNORED_DIRECTORIES.iter().any(|ignored| name.eq_ignore_ascii_case(ignored)) {
                return None;
            }
            Some((path, name, file_type.is_dir()))
        })
        .take(remaining.saturating_add(1))
        .collect::<Vec<_>>();
    if paths.len() > remaining {
        paths.truncate(remaining);
        *truncated = true;
    }
    paths.sort_by(|a, b| {
        b.2.cmp(&a.2).then_with(|| a.1.to_lowercase().cmp(&b.1.to_lowercase()))
    });

    let mut nodes = Vec::new();
    for (path, name, is_dir) in paths {
        if cancelled.load(Ordering::Relaxed) {
            break;
        }
        if *node_count >= MAX_NODES {
            *truncated = true;
            break;
        }

        *node_count += 1;
        let children = if is_dir {
            scan_directory(&path, depth + 1, cancelled, all_files, node_count, truncated)
        } else {
            all_files.push(path.clone());
            Vec::new()
        };

        nodes.push(FileNode { path, name, is_dir, children });
    }
    nodes
}

pub fn flatten_visible(
    nodes: &[FileNode],
    expanded: &std::collections::HashSet<PathBuf>,
) -> Vec<TreeRow> {
    let mut output = Vec::new();
    for node in nodes {
        flatten_node(node, 0, expanded, &mut output);
    }
    output
}

fn flatten_node(
    node: &FileNode,
    depth: usize,
    expanded: &std::collections::HashSet<PathBuf>,
    output: &mut Vec<TreeRow>,
) {
    output.push(TreeRow {
        path: node.path.clone(),
        name: node.name.clone(),
        is_dir: node.is_dir,
        depth,
    });

    if node.is_dir && expanded.contains(&node.path) {
        for child in &node.children {
            flatten_node(child, depth + 1, expanded, output);
        }
    }
}
