use serde::{Serialize, de::DeserializeOwned};
use wasm_bindgen::JsValue;
use crate::{Result, scope::window};

/// Interface that provides access to a particular domain's session or local storage.
/// 
/// It allows, for example, the addition, modification, or deletion of stored data item
#[derive(Clone)]
pub struct Storage {
    inner: web_sys::Storage
}

impl Storage {
    /// Returns a [`Storage`] instance over local storage
    #[inline]
    pub fn local () -> Result<Option<Self>> {
        return Ok(window()?.local_storage()?.map(|inner| Self { inner }))
    }

    /// Returns a [`Storage`] instance over session storage
    #[inline]
    pub fn session () -> Result<Option<Self>> {
        return Ok(window()?.session_storage()?.map(|inner| Self { inner }))
    }
    
    /// Returns the number of entries of the storage
    #[inline]
    pub fn len (&self) -> Result<usize> {
        return Ok(self.inner.length()? as usize)
    }

    /// Sets the serialized value into the store.
    pub fn set<T: Serialize> (&self, key: &str, value: &T) -> Result<()> {
        let value = match serde_json::to_string(value) {
            Ok(x) => x,
            Err(e) => return Err(JsValue::from_str(&e.to_string()))
        };
        return self.inner.set_item(key, &value);
    }

    /// Returns the deserialized value from the store.
    pub fn get<T: DeserializeOwned> (&self, key: &str) -> Result<Option<T>> {
        if let Some(str) = self.inner.get_item(key)? {
            return match serde_json::from_str(&str) {
                Ok(x) => Ok(x),
                Err(e) => Err(JsValue::from_str(&e.to_string()))
            }
        }
        return Ok(None)
    }

    /// Removes the value associated to the specified key from the store
    #[inline]
    pub fn remove (&self, key: &str) -> Result<()> {
        self.inner.remove_item(key)
    }

    /// Removes all the entries from the store.
    #[inline]
    pub fn clear (&self) -> Result<()> {
        self.inner.clear()
    }

    /// Returns an iterator over all of the entries of the store
    #[inline]
    pub fn iter (&self) -> StorageIter {
        let len = match self.inner.length() {
            Ok(len) => len,
            Err(_) => {
                let mut i = 0;
                while self.inner.key(i).unwrap().is_some() {
                    i += 1;
                }
                i
            }
        };

        return StorageIter {
            inner: self.inner.clone(),
            front: 0,
            back: len - 1,
            len
        }
    }
}

impl IntoIterator for Storage {
    type Item = Result<(String, String)>;
    type IntoIter = StorageIter;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl IntoIterator for &Storage {
    type Item = Result<(String, String)>;
    type IntoIter = StorageIter;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

/// Iterator over [`Storage`]
#[derive(Debug)]
pub struct StorageIter {
    inner: web_sys::Storage,
    front: u32,
    back: u32,
    len: u32
}

impl StorageIter {
    /// Returns the next value of the iterator deserialized
    #[inline]
    pub fn next_value<T: DeserializeOwned> (&mut self) -> Option<Result<(String, T)>> {
        let (key, value) = match self.next()? {
            Ok(x) => x,
            Err(e) => return Some(Err(e))
        };

        return match serde_json::from_str(&value) {
            Ok(x) => Some(Ok((key, x))),
            Err(e) => Some(Err(JsValue::from_str(&e.to_string())))
        };
    }

    /// Returns the nth value of the iterator deserialized
    #[inline]
    pub fn nth_value<T: DeserializeOwned> (&mut self, n: usize) -> Option<Result<(String, T)>> {
        self.front += u32::try_from(n).unwrap();
        self.next_value()
    }
}

impl Iterator for StorageIter {
    type Item = Result<(String, String)>;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        macro_rules! tri {
            ($e:expr) => {
                match $e {
                    Ok(x) => x,
                    Err(e) => return Some(Err(e))
                }
            };
        }

        if let Some(key) = tri!(self.inner.key(self.front)) {
            let value = unsafe { tri!(self.inner.get_item(&key)).unwrap_unchecked() };
            self.front += 1;
            return Some(Ok((key, value)))
        }

        return None
    }

    #[inline]
    fn nth(&mut self, n: usize) -> Option<Self::Item> {
        self.front += u32::try_from(n).unwrap();
        self.next()
    }

    #[inline]
    fn count(self) -> usize where Self: Sized, {
        return (self.len - self.front) as usize
    }

    #[inline]
    fn last(mut self) -> Option<Self::Item> where Self: Sized, {
        return self.nth((self.len - self.front - 1) as usize)
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        let len = self.len as usize;
        return (len, Some(len))
    }
}

impl DoubleEndedIterator for StorageIter {
    #[inline]
    fn next_back(&mut self) -> Option<Self::Item> {
        macro_rules! tri {
            ($e:expr) => {
                match $e {
                    Ok(x) => x,
                    Err(e) => return Some(Err(e))
                }
            };
        }

        if let Some(key) = tri!(self.inner.key(self.back)) {
            let value = unsafe { tri!(self.inner.get_item(&key)).unwrap_unchecked() };
            self.back -= 1;
            return Some(Ok((key, value)))
        }

        return None
    }

    #[inline]
    fn nth_back(&mut self, n: usize) -> Option<Self::Item> {
        self.back -= u32::try_from(n).unwrap();
        self.next()
    }
}

impl ExactSizeIterator for StorageIter {
    #[inline]
    fn len(&self) -> usize {
        self.len as usize
    }
}