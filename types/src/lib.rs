use rand::Rng;

pub type StorageId = u32;

pub struct Storage<T> {
    items: std::collections::HashMap<StorageId, T>,
}

impl<T> Storage<T> {
    pub fn new() -> Self {
        Storage {
            items: std::collections::HashMap::new(),
        }
    }

    pub fn insert(&mut self, value: T) -> StorageId {
        let mut rng = rand::rng();
        let mut id: StorageId;

        loop {
            id = rng.random::<StorageId>();
            if !self.items.contains_key(&id) {
                break;
            }
        }

        self.items.insert(id, value);
        id
    }

    #[inline]
    pub fn get(&self, id: &StorageId) -> Option<&T> {
        self.items.get(&id)
    }

    #[inline]
    pub fn get_mut(&mut self, id: &StorageId) -> Option<&mut T> {
        self.items.get_mut(&id)
    }

    #[inline]
    pub fn remove(&mut self, id: &StorageId) -> Option<T> {
        self.items.remove(id)
    }

    pub fn iter(&self) -> impl Iterator<Item = (&StorageId, &T)> {
        self.items.iter()
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = (&StorageId, &mut T)> {
        self.items.iter_mut()
    }
}

impl<'a, T> IntoIterator for &'a mut Storage<T> {
    type Item = (&'a StorageId, &'a mut T);
    type IntoIter = std::collections::hash_map::IterMut<'a, StorageId, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.items.iter_mut()
    }
}

impl<T> IntoIterator for Storage<T> {
    type Item = (StorageId, T);
    type IntoIter = std::collections::hash_map::IntoIter<StorageId, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.items.into_iter()
    }
}

impl<T> Default for Storage<T> {
    fn default() -> Self {
        Self {
            items: std::collections::HashMap::new(),
        }
    }
}

pub struct Array2d<T> {
    col_count: usize,
    row_count: usize,
    array: Box<[T]>,
}

impl<T: Clone> Array2d<T> {
    #[inline]
    pub fn new(rows: usize, cols: usize, default_value: T) {
        Self {
            col_count: cols,
            row_count: rows,
            array: vec![default_value; rows * cols].into_boxed_slice(),
        };
    }

    pub fn get(&self, c: usize, r: usize) -> Option<&T> {
        if c >= self.col_count || r >= self.row_count {
            return None;
        }

        self.array.get((r * self.col_count) + c)
    }

    pub fn get_mut(&mut self, c: usize, r: usize) -> Option<&mut T> {
        if c >= self.col_count || r >= self.row_count {
            return None;
        }

        self.array.get_mut((r * self.col_count) + c)
    }

    #[inline]
    pub fn len(&self) -> (usize, usize) {
        (self.col_count, self.row_count)
    }
}
