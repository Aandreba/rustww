use std::{time::{SystemTime, Duration}, fmt::Debug};
use chrono::{DateTime, Utc, NaiveDateTime};
use js_sys::{Array};
use wasm_bindgen::{prelude::wasm_bindgen, JsCast, JsValue};
use wasm_bindgen_futures::JsFuture;
use web_sys::Window;
use crate::{Result, window, io::JsReadStream};

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_name = File, extends = web_sys::Blob)]
    #[derive(Debug, Clone, PartialEq)]
    type JsFile;
    #[derive(Debug, Clone, PartialEq)]
    type FileSystemFileHandle;
    #[derive(Debug, Clone, PartialEq)]
    type FileSystemWritableFileStream;

    #[wasm_bindgen(method, getter)]
    fn name (this: &JsFile) -> String;
    #[wasm_bindgen(method, getter)]
    fn size (this: &JsFile) -> u32;
    #[wasm_bindgen(method, getter, js_name = lastModified)]
    fn last_modified (this: &JsFile) -> i32;

    #[wasm_bindgen(method, js_name = getFile)]
    fn get_file (this: &FileSystemFileHandle) -> js_sys::Promise;
    #[wasm_bindgen(method, js_name = createWritable)]
    fn create_writable (this: &FileSystemFileHandle) -> js_sys::Promise;

    #[wasm_bindgen(js_namespace = window, js_name = showOpenFilePicker)]
    fn show_open_file_picker (this: &Window) -> js_sys::Promise;
}

#[derive(Debug)]
pub struct File {
    inner: FileSystemFileHandle,
    read: Option<JsFile>,
    write: Option<FileSystemWritableFileStream>
}

impl File {
    pub async fn from_picker () -> Result<impl Iterator<Item = File>> {
        let picker = JsFuture::from(show_open_file_picker(&window()?)).await?;
        let array = picker.unchecked_into::<Array>();

        let iter = (0..array.length()).into_iter()
            .map(move |i| File {
                inner: array.get(i).unchecked_into::<FileSystemFileHandle>(),
                read: None,
                write: None
            });

        return Ok(iter)
    }

    #[inline]
    pub async fn metadata (&mut self) -> Result<Metadata> {
        let read = self.get_read().await?;
        return Ok(Metadata { len: read.size() as u64, last_modified: read.last_modified() })
    }

    #[inline]
    pub async fn read_stream (&mut self) -> Result<JsReadStream> {
        let read = self.get_read().await?;
        return JsReadStream::new(read.stream());
    }

    async fn get_read (&mut self) -> Result<&JsFile> {
        if let Some(ref read) = self.read {
            return Ok(read)
        }

        let read = JsFuture::from(self.inner.get_file())
            .await?
            .unchecked_into::<JsFile>();
        
        self.read = Some(read);
        return Ok(unsafe { self.read.as_ref().unwrap_unchecked() })
    }

    async fn get_write (&mut self) -> Result<&FileSystemWritableFileStream> {
        if let Some(ref write) = self.write {
            return Ok(write)
        }

        let write = JsFuture::from(self.inner.create_writable())
            .await?
            .unchecked_into::<FileSystemWritableFileStream>();
        
        self.write = Some(write);
        return Ok(unsafe { self.write.as_ref().unwrap_unchecked() })
    }
}

#[derive(Clone)]
pub struct Metadata {
    len: u64,
    last_modified: i32
}

impl Metadata {
    #[inline]
    pub fn len (&self) -> u64 {
        return self.len
    }

    #[inline]
    pub fn modified_millis (&self) -> i32 {
        return self.last_modified
    }

    #[inline]
    pub fn modified (&self) -> Result<SystemTime> {
        let time = match self.last_modified {
            x if x.is_negative() => SystemTime::UNIX_EPOCH.checked_sub(Duration::from_millis(x.unsigned_abs() as u64)),
            x => SystemTime::UNIX_EPOCH.checked_add(Duration::from_millis(x.unsigned_abs() as u64))
        };

        return time.ok_or_else(|| JsValue::from_str("unsupported timestamp"))
    }

    #[inline]
    pub fn modified_date_js (&self) -> js_sys::Date {
        return js_sys::Date::new(&JsValue::from(self.last_modified))
    }

    #[inline]
    pub fn modified_date (&self) -> Result<DateTime<Utc>> {
        let naive = NaiveDateTime::from_timestamp_millis(self.last_modified as i64)
            .ok_or_else(|| JsValue::from_str("unsupported timestamp"))?;
        return Ok(DateTime::from_utc(naive, Utc));
    }
}

impl Debug for Metadata {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Metadata")
            .field("len", &self.len)
            .field("modified", &self.last_modified)
            .finish()
    }
}