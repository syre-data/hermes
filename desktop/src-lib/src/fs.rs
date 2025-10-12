use serde::{Deserialize, Serialize};
use std::{collections::BTreeSet, ffi::OsString};

#[cfg(feature = "fs")]
use std::{fs, path::Path};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Directory {
    #[serde(with = "serde_os_string")]
    pub name: OsString,
    #[serde(with = "serde_os_string_seq")]
    pub files: BTreeSet<OsString>,
}

impl Directory {
    pub fn new(name: impl Into<OsString>) -> Self {
        Self {
            name: name.into(),
            files: BTreeSet::new(),
        }
    }

    pub fn new_with_files(
        name: impl Into<OsString>,
        files: impl IntoIterator<Item = OsString>,
    ) -> Self {
        Self {
            name: name.into(),
            files: BTreeSet::from_iter(files),
        }
    }
}

/// Directory tree graph.
#[derive(Serialize, Deserialize, Clone)]
pub struct DirectoryTree {
    directories: Vec<Directory>,

    /// Parent of directory at index `i + 1`.
    /// Value for graph root is not included.
    parents: Vec<usize>,
}

impl DirectoryTree {
    pub const ROOT: usize = 0;

    pub fn new(root: Directory) -> Self {
        Self {
            directories: vec![root],
            parents: vec![],
        }
    }

    /// Insert a new directory into the graph.
    ///
    /// # Returns
    /// Index of newly inserted directory.
    /// `Err` if `parent` does not exist.
    pub fn insert(&mut self, directory: Directory, parent: usize) -> Result<usize, ()> {
        debug_assert_eq!(self.directories.len() - 1, self.parents.len());

        if parent >= self.directories.len() {
            return Err(());
        }

        let idx = self.directories.len();
        self.directories.push(directory);
        self.parents.push(parent);
        Ok(idx)
    }

    /// Remove a subgraph.
    ///
    /// # Returns
    /// Removed subgraph.
    /// `Err` if `root` does not exist.
    pub fn remove(&mut self, root: usize) -> Result<DirectoryTree, error::Remove> {
        if root == Self::ROOT {
            return Err(error::Remove::GraphRoot);
        }
        if root >= self.directories.len() {
            return Err(error::Remove::InvalidRoot);
        }

        let mut descendants = self.descendants(root);
        debug_assert!(!descendants.is_empty());
        descendants.sort();
        let descendants = descendants.into_iter().rev();

        let mut directories = vec![];
        let mut parents = vec![];
        let mut parent_map = vec![];
        let mut root_idx = 0;
        for descendant in descendants {
            let directory = self.directories.swap_remove(descendant);
            let parent = self.parents.swap_remove(descendant - 1);
            directories.push(directory);
            parents.push(parent);
            parent_map.push((parent, directories.len() - 1));
            if descendant == root {
                root_idx = directories.len() - 1;
            }

            let swap_idx = self.directories.len();
            for parent in self.parents.iter_mut() {
                if swap_idx == *parent {
                    *parent = descendant;
                }
            }
        }
        debug_assert_eq!(self.directories.len() - 1, self.parents.len());
        debug_assert_eq!(directories.len(), parents.len());

        directories.swap(0, root_idx);
        parents.swap(0, root_idx);
        parents.remove(0);

        for (from, to) in parent_map {
            for parent in parents.iter_mut() {
                if from == *parent {
                    *parent = to;
                }
            }
        }

        Ok(Self {
            directories,
            parents,
        })
    }

    /// Move a subgraph to a new parent.
    ///
    /// # Returns
    /// `Err` if `root` or `parent` are invalid.
    pub fn shift(&mut self, root: usize, parent: usize) -> Result<(), error::Shift> {
        if root >= self.directories.len() {
            return Err(error::Shift::InvalidRoot);
        }
        if parent >= self.directories.len() {
            return Err(error::Shift::InvalidParent);
        }

        let descendants = self.descendants(root);
        if descendants.contains(&parent) {
            return Err(error::Shift::CanNotShiftToDescendant);
        }

        self.parents[root - 1] = parent;
        Ok(())
    }

    /// # Returns
    /// All directories.
    pub fn directories(&self) -> &Vec<Directory> {
        &self.directories
    }

    /// # Returns
    /// Directory at the given index.
    pub fn get(&self, idx: usize) -> Result<&Directory, error::NodeDoesNotExist> {
        self.directories.get(idx).ok_or(error::NodeDoesNotExist)
    }

    /// # Returns
    /// Directory at the given index.
    pub fn get_mut(&mut self, idx: usize) -> Result<&mut Directory, error::NodeDoesNotExist> {
        self.directories.get_mut(idx).ok_or(error::NodeDoesNotExist)
    }

    /// # Returns
    /// Children indices of the directory at the given index.
    /// `Err` if `parent` is invalid.
    pub fn children(&self, parent: usize) -> Result<Vec<usize>, error::NodeDoesNotExist> {
        if parent >= self.directories.len() {
            return Err(error::NodeDoesNotExist);
        }

        Ok(self
            .parents
            .iter()
            .enumerate()
            .filter_map(|(child, c_parent)| (*c_parent == parent).then_some(child + 1))
            .collect())
    }

    /// # Returns
    /// Parents array.
    /// Value at index `i` is the index of the parent for the directory at index `(i + 1)`.
    ///
    /// # Notes
    /// + Parent indices are not stable across write operations.
    pub fn parents(&self) -> &Vec<usize> {
        &self.parents
    }

    /// # Returns
    /// Parent of `child`.
    /// `None` if `child` is the root.
    ///
    /// # Notes
    /// + Parent indices are not stable across write operations.
    pub fn parent(&self, child: usize) -> Result<Option<usize>, error::NodeDoesNotExist> {
        if child >= self.directories.len() {
            return Err(error::NodeDoesNotExist);
        }
        if child == Self::ROOT {
            return Ok(None);
        }

        Ok(Some(self.parents[child - 1]))
    }

    /// # Returns
    /// All descendants of `root` including `root` itself.
    /// `root` is at index `0`.
    /// If the returned `Vec` is empty, it indicates `root` does not exist in the graph.
    fn descendants(&self, root: usize) -> Vec<usize> {
        if root >= self.directories.len() {
            return vec![];
        }

        let mut descendants = vec![root];
        let mut remaining_children = self.children(root).unwrap();
        while let Some(child) = remaining_children.pop() {
            descendants.push(child);
            remaining_children.extend(self.children(child).unwrap().iter());
        }

        descendants
    }

    /// # Returns
    /// Ordered list of `roots` ancestors.
    /// `root` is at index `0`, Graph root is the last element.
    /// If the returned `Vec` is empty, it indicates `root` does not exist in the graph.
    fn ancestors(&self, root: usize) -> Vec<usize> {
        if root >= self.directories.len() {
            return vec![];
        }

        let mut root = root;
        let mut ancestors = vec![root];
        while root != Self::ROOT {
            let parent = self.parents[root - 1];
            ancestors.push(parent);
            root = parent;
        }

        ancestors
    }

    /// # Returns
    /// Path components to `root`.
    /// If the returned `Vec` is empty, it indicates `root` does not exist in the graph.
    pub fn path(&self, root: usize) -> Vec<OsString> {
        self.ancestors(root)
            .into_iter()
            .rev()
            .map(|ancestor| self.get(ancestor).unwrap().name.clone())
            .collect()
    }
}

#[cfg(feature = "fs")]
impl DirectoryTree {
    /// Create a `DirectoryTree` from a file system path.
    pub fn from_file_system(path: impl AsRef<Path>) -> Result<Self, error::FromFileSystem> {
        use std::collections::VecDeque;

        let path = path.as_ref();
        if !path.exists() {
            return Err(error::FromFileSystem::RootNotFound);
        }
        if !path.is_dir() {
            return Err(error::FromFileSystem::RootNotADirectory);
        }

        let mut directories = vec![];
        let mut parents = vec![];
        let mut parent_map = vec![];
        let mut is_root = true;
        let mut unexplored = VecDeque::new();
        unexplored.push_back(path.to_path_buf());
        while let Some(active) = unexplored.pop_front() {
            let name = active
                .file_name()
                .map(|name| name.to_os_string())
                .unwrap_or("/".into());

            let entries = fs::read_dir(&active)
                .map_err(|err| error::FromFileSystem::ReadDir {
                    path: active.clone(),
                    error: err.kind(),
                })?
                .filter_map(|entry| entry.ok())
                .collect::<Vec<_>>();

            let children = entries
                .iter()
                .filter_map(|entry| {
                    entry
                        .file_type()
                        .ok()
                        .map(|kind| kind.is_dir().then_some(entry.path()))
                        .flatten()
                })
                .collect::<Vec<_>>();
            let files = entries.iter().filter_map(|entry| {
                entry
                    .file_type()
                    .ok()
                    .map(|kind| kind.is_file().then_some(entry.file_name()))
                    .flatten()
            });

            unexplored.extend(children.iter().cloned());
            directories.push(Directory::new_with_files(name, files));

            if !is_root {
                let parent_map_idx = parent_map
                    .iter()
                    .position(|(child, _)| *child == active)
                    .unwrap();
                let (_, parent) = parent_map.remove(parent_map_idx);
                parents.push(parent);
            } else {
                is_root = false;
            }

            let parent_idx = directories.len() - 1;
            let children = children.into_iter().map(|child| (child, parent_idx));
            parent_map.extend(children);
        }

        Ok(Self {
            directories,
            parents,
        })
    }
}

pub mod error {
    use serde::{Deserialize, Serialize};
    use std::{io, path::PathBuf};

    #[derive(Serialize, Deserialize, Copy, Clone, Debug)]
    pub struct NodeDoesNotExist;

    #[derive(Serialize, Deserialize, Copy, Clone, Debug)]
    pub enum Remove {
        /// Can not remove the graph's root.
        GraphRoot,

        /// Root does not exist.
        InvalidRoot,
    }

    #[derive(Serialize, Deserialize, Copy, Clone, Debug)]
    pub enum Shift {
        /// Root node does not exist.
        InvalidRoot,

        /// Parent node does not exist.
        InvalidParent,

        /// Attempt to adopt root into one of its decendants.
        /// i.e. The new parent is a descendant of the root or the root itself.
        CanNotShiftToDescendant,
    }

    #[derive(Serialize, Deserialize, Debug, thiserror::Error, Clone)]
    pub enum FromFileSystem {
        /// Root resource was not found.
        #[error("Could not find the project root.")]
        RootNotFound,

        /// Root resource is not a directory.
        #[error("Project root is not a directory.")]
        RootNotADirectory,

        #[error("Could not read path `{path:?}` [{error:?}]")]
        ReadDir {
            path: PathBuf,

            #[serde(with = "io_error_serde::ErrorKind")]
            error: io::ErrorKind,
        },
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn directory_tree() {
        let root_name = "0";
        let c0_name = "0.0";
        let c1_name = "0.1";
        let c00_name = "0.0.0";
        let c10_name = "0.1.0";
        let root = Directory::new(root_name);
        let c0 = Directory::new(c0_name);
        let c00 = Directory::new(c00_name);
        let c1 = Directory::new(c1_name);
        let c10 = Directory::new(c10_name);

        let mut tree = DirectoryTree::new(root);
        let c0_idx = tree.insert(c0, DirectoryTree::ROOT).unwrap();
        let c1_idx = tree.insert(c1, DirectoryTree::ROOT).unwrap();
        let c00_idx = tree.insert(c00, c0_idx).unwrap();
        let c10_idx = tree.insert(c10, c1_idx).unwrap();

        assert_eq!(tree.parent(c0_idx).unwrap().unwrap(), DirectoryTree::ROOT);
        assert_eq!(tree.parent(c1_idx).unwrap().unwrap(), DirectoryTree::ROOT);
        assert_eq!(tree.parent(c00_idx).unwrap().unwrap(), c0_idx);
        assert_eq!(tree.parent(c10_idx).unwrap().unwrap(), c1_idx);
        assert_eq!(tree.descendants(c10_idx), vec![c10_idx]);
        assert_eq!(tree.descendants(c1_idx), vec![c1_idx, c10_idx]);
        assert_eq!(
            tree.ancestors(DirectoryTree::ROOT),
            vec![DirectoryTree::ROOT]
        );
        assert_eq!(
            tree.ancestors(c00_idx),
            vec![c00_idx, c0_idx, DirectoryTree::ROOT]
        );
        assert_eq!(
            tree.ancestors(c10_idx),
            vec![c10_idx, c1_idx, DirectoryTree::ROOT]
        );

        tree.shift(c1_idx, c0_idx).unwrap();
        assert_eq!(tree.parent(c1_idx).unwrap().unwrap(), c0_idx);
        assert_eq!(
            tree.ancestors(c10_idx),
            vec![c10_idx, c1_idx, c0_idx, DirectoryTree::ROOT]
        );

        let c1_tree = tree.remove(c1_idx).unwrap();
        assert_eq!(c1_tree.get(DirectoryTree::ROOT).unwrap().name, c1_name);
        assert_eq!(c1_tree.directories().len(), 2);
        let c1_children = c1_tree.children(DirectoryTree::ROOT).unwrap();
        assert_eq!(c1_children.len(), 1);
        let c10_idx = c1_children[0];
        assert_eq!(c1_tree.get(c10_idx).unwrap().name, c10_name);
        assert_eq!(tree.directories().len(), 3);
        assert_eq!(tree.get(DirectoryTree::ROOT).unwrap().name, root_name);
        let root_children = tree.children(DirectoryTree::ROOT).unwrap();
        assert_eq!(root_children.len(), 1);
        let c0_idx = root_children[0];
        assert_eq!(tree.get(c0_idx).unwrap().name, c0_name);
        let c0_children = tree.children(c0_idx).unwrap();
        assert_eq!(c0_children.len(), 1);
        let c00_idx = c0_children[0];
        assert_eq!(tree.get(c00_idx).unwrap().name, c00_name);
    }
}

pub mod serde_os_string {
    use serde::{Deserializer, Serializer, de::Visitor};
    use std::{ffi::OsString, fmt, str::FromStr};

    pub fn serialize<S>(value: &OsString, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&value.to_string_lossy().to_string())
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<OsString, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_string(OsStringVisitor)
    }

    struct OsStringVisitor;
    impl<'de> Visitor<'de> for OsStringVisitor {
        type Value = OsString;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("os string")
        }

        fn visit_string<E>(self, v: String) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            OsString::from_str(&v).map_err(|err| serde::de::Error::custom(format!("{err:?}")))
        }

        fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            OsString::from_str(&v).map_err(|err| serde::de::Error::custom(format!("{err:?}")))
        }
    }
}

pub mod serde_os_string_seq {
    use serde::{Deserializer, Serializer, de::Visitor, ser::SerializeSeq};
    use std::{collections::BTreeSet, ffi::OsString, fmt, str::FromStr};

    pub fn serialize<S>(value: &BTreeSet<OsString>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut seq = serializer.serialize_seq(Some(value.len()))?;
        for item in value {
            let os_str = item.to_string_lossy();
            seq.serialize_element(&os_str)?;
        }
        seq.end()
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<BTreeSet<OsString>, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_seq(OsStringSeqVisitor)
    }

    struct OsStringSeqVisitor;
    impl<'de> Visitor<'de> for OsStringSeqVisitor {
        type Value = BTreeSet<OsString>;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("sequence of os strings")
        }

        fn visit_seq<S>(self, mut seq: S) -> Result<Self::Value, S::Error>
        where
            S: serde::de::SeqAccess<'de>,
        {
            let mut items = if let Some(len) = seq.size_hint() {
                Vec::with_capacity(len)
            } else {
                Vec::new()
            };

            while let Some(item) = seq.next_element::<String>()? {
                let os_str = OsString::from_str(&item)
                    .map_err(|err| serde::de::Error::custom(format!("{err:?}")))?;
                items.push(os_str);
            }

            Ok(BTreeSet::from_iter(items))
        }
    }
}
