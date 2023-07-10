use std::{ops::{Deref, DerefMut}, path::{Path, PathBuf}};

use indexmap::IndexMap;
use tokio::fs;

use super::File;
use crate::{config::{manager::SortBy, MANAGER}, emit};

#[derive(Default)]
pub struct Files {
	items:           IndexMap<PathBuf, File>,
	sort:            FilesSort,
	pub show_hidden: bool,
}

impl Files {
	pub async fn from(paths: Vec<PathBuf>) -> IndexMap<PathBuf, File> {
		let mut items = IndexMap::new();
		for path in paths {
			if let Ok(file) = File::from(&path).await {
				items.insert(path, file);
			}
		}
		items
	}

	pub async fn read(path: &Path) {
		let mut iter = match fs::read_dir(path).await {
			Ok(it) => it,
			Err(_) => return,
		};

		let mut items = IndexMap::new();
		while let Ok(Some(item)) = iter.next_entry().await {
			if let Ok(meta) = item.metadata().await {
				let path = item.path();
				let file = File::from_meta(&path, meta).await;
				items.insert(path, file);
			}
		}
		emit!(Files(FilesOp::Update(path.to_path_buf(), items)));
	}

	pub fn sort(&mut self) {
		fn cmp<T: Ord>(a: T, b: T, reverse: bool) -> std::cmp::Ordering {
			if reverse { b.cmp(&a) } else { a.cmp(&b) }
		}

		let reverse = self.sort.reverse;
		match self.sort.by {
			SortBy::Alphabetical => self.items.sort_by(|_, a, _, b| cmp(&a.name, &b.name, reverse)),
			SortBy::Created => self.items.sort_by(|_, a, _, b| {
				if let (Ok(a), Ok(b)) = (a.meta.created(), b.meta.created()) {
					return cmp(a, b, reverse);
				}
				std::cmp::Ordering::Equal
			}),
			SortBy::Modified => self.items.sort_by(|_, a, _, b| {
				if let (Ok(a), Ok(b)) = (a.meta.modified(), b.meta.modified()) {
					return cmp(a, b, reverse);
				}
				std::cmp::Ordering::Equal
			}),
			SortBy::Size => {
				self.items.sort_by(|_, a, _, b| cmp(a.length.unwrap_or(0), b.length.unwrap_or(0), reverse))
			}
		}
	}

	pub fn update(&mut self, mut items: IndexMap<PathBuf, File>) -> bool {
		if !self.show_hidden {
			items.retain(|_, item| !item.is_hidden);
		}

		for (path, item) in &mut items {
			if let Some(old) = self.items.get(path) {
				item.length = old.length;
				item.is_selected = old.is_selected;
			}
		}

		self.items = items;
		self.sort();
		true
	}

	pub fn append(&mut self, items: IndexMap<PathBuf, File>) -> bool {
		for (path, mut item) in items.into_iter() {
			if let Some(old) = self.items.get(&path) {
				item.length = old.length;
				item.is_selected = old.is_selected;
			}
			self.items.insert(path, item);
		}

		self.sort();
		true
	}
}

impl Deref for Files {
	type Target = IndexMap<PathBuf, File>;

	fn deref(&self) -> &Self::Target { &self.items }
}

impl DerefMut for Files {
	fn deref_mut(&mut self) -> &mut Self::Target { &mut self.items }
}

struct FilesSort {
	pub by:      SortBy,
	pub reverse: bool,
}

impl Default for FilesSort {
	fn default() -> Self { Self { by: MANAGER.sort_by, reverse: MANAGER.sort_reverse } }
}

pub enum FilesOp {
	Update(PathBuf, IndexMap<PathBuf, File>),
	Append(PathBuf, IndexMap<PathBuf, File>),
}

impl FilesOp {
	#[inline]
	pub fn path(&self) -> PathBuf {
		match self {
			Self::Update(path, _) => path,
			Self::Append(path, _) => path,
		}
		.clone()
	}
}