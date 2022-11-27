use serde::{Serialize, de::DeserializeOwned};
use wasm_bindgen::JsValue;
use crate::{Result, window};

#[derive(Clone)]
pub struct Storage {
    inner: web_sys::Storage
}

impl Storage {
    #[inline]
    pub fn local () -> Result<Option<Self>> {
        return Ok(window()?.local_storage()?.map(|inner| Self { inner }))
    }

    #[inline]
    pub fn session () -> Result<Option<Self>> {
        return Ok(window()?.session_storage()?.map(|inner| Self { inner }))
    }
    
    #[inline]
    pub fn len (&self) -> Result<usize> {
        return Ok(self.inner.length()? as usize)
    }

    pub fn set<T: Serialize> (&self, key: &str, value: &T) -> Result<()> {
        let value = match serde_json::to_string(value) {
            Ok(x) => x,
            Err(e) => return Err(JsValue::from_str(&e.to_string()))
        };

        return self.inner.set_item(key, &value);
    }

    pub fn get<T: DeserializeOwned> (&self, key: &str) -> Result<Option<T>> {
        if let Some(str) = self.inner.get_item(key)? {
            return match serde_json::from_str(&str) {
                Ok(x) => Ok(x),
                Err(e) => Err(JsValue::from_str(&e.to_string()))
            }
        }
        return Ok(None)
    }

    #[inline]
    pub fn remove (&self, key: &str) -> Result<()> {
        self.inner.remove_item(key)
    }

    #[inline]
    pub fn clear (&self) -> Result<()> {
        self.inner.clear()
    }

    #[inline]
    pub fn iter (&self) -> StorageIter {
        return StorageIter { inner: self.inner.clone(), idx: 0 }
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

#[derive(Debug)]
pub struct StorageIter {
    inner: web_sys::Storage,
    idx: u32
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

        if let Some(key) = tri!(self.inner.key(self.idx)) {
            let value = unsafe { tri!(self.inner.get_item(&key)).unwrap_unchecked() };
            self.idx += 1;
            return Some(Ok((key, value)))
        }

        return None
    }

    #[inline]
    fn nth(&mut self, n: usize) -> Option<Self::Item> {
        self.idx += u32::try_from(n).unwrap();
        self.next()
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let len = match self.inner.length() {
            Ok(len) => len - self.idx,
            Err(_) => return (0, None)
        } as usize;
        (len, Some(len))
    }
}