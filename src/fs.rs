use std::{time::{SystemTime, Duration}, fmt::Debug, ops::{Deref}, io::SeekFrom};
use chrono::{DateTime, Utc, NaiveDateTime};
use js_sys::{Array};
use wasm_bindgen::{prelude::wasm_bindgen, JsCast, JsValue};
use wasm_bindgen_futures::JsFuture;
use web_sys::Window;
use crate::{Result, window, io::{JsReadStream, JsWriteStream}};

/// File reading/writing permissions granted by the user
#[wasm_bindgen]
#[non_exhaustive]
pub enum PermisionStatus {
    Granted = "granted",
    Denied = "denied",
    Prompt = "prompt"
}

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_name = File, extends = web_sys::Blob)]
    #[derive(Debug, Clone, PartialEq)]
    type JsFile;
    #[derive(Debug, Clone, PartialEq)]
    type FileSystemHandle;
    #[wasm_bindgen(extends = FileSystemHandle)]
    #[derive(Debug, Clone, PartialEq)]
    type FileSystemFileHandle;
    #[wasm_bindgen(extends = web_sys::WritableStream)]
    #[derive(Debug, Clone, PartialEq)]
    type FileSystemWritableFileStream;

    #[wasm_bindgen(method, getter)]
    fn name (this: &JsFile) -> String;
    #[wasm_bindgen(method, getter)]
    fn size (this: &JsFile) -> u32;
    #[wasm_bindgen(method, getter, js_name = lastModified)]
    fn last_modified (this: &JsFile) -> i32;

    #[wasm_bindgen(method, catch, js_name = queryPermission)]
    fn query_permission (this: &FileSystemHandle, ops: &JsValue) -> Result<PermisionStatus>;
    #[wasm_bindgen(method, catch, js_name = requestPermission)]
    fn request_permission (this: &FileSystemHandle, ops: &JsValue) -> Result<js_sys::Promise>;

    #[wasm_bindgen(method, js_name = getFile)]
    fn get_file (this: &FileSystemFileHandle) -> js_sys::Promise;
    #[wasm_bindgen(method, js_name = createWritable)]
    fn create_writable (this: &FileSystemFileHandle) -> js_sys::Promise;

    #[wasm_bindgen(method)]
    fn seek (this: &FileSystemWritableFileStream, pos: u64) -> js_sys::Promise;
    #[wasm_bindgen(method)]
    fn truncate (this: &FileSystemWritableFileStream, pos: u64) -> js_sys::Promise;
    
    #[wasm_bindgen(js_namespace = window, js_name = showOpenFilePicker)]
    fn show_open_file_picker (this: &Window) -> js_sys::Promise;
}

/// Object that provides acces to a file on the filesystem.
/// 
/// An instance of [`File`] can (currently) only be created via a [file picker](File::from_picker)
/// 
/// # Compatibility
/// Check the [compatibility table](https://developer.mozilla.org/en-US/docs/Web/API/FileSystemFileHandle)
#[derive(Debug)]
pub struct File {
    inner: FileSystemFileHandle
}

impl File {
    /// Creates new instances of [`File`] from the files returned by the file selector
    pub async fn from_picker () -> Result<impl Iterator<Item = File>> {
        let picker = JsFuture::from(show_open_file_picker(&window()?)).await?;
        let array = picker.unchecked_into::<Array>();

        let iter = (0..array.length()).into_iter()
            .map(move |i| File {
                inner: array.get(i).unchecked_into::<FileSystemFileHandle>()
            });

        return Ok(iter)
    }

    /// Returns the file's metadata
    #[inline]
    pub async fn metadata (&mut self) -> Result<Metadata> {
        let read = self.get_read().await?;
        return Ok(Metadata { len: read.size() as u64, last_modified: read.last_modified() })
    }

    /// Returns a [`JsReadStream`] that can be used to read the contents of the file
    #[inline]
    pub async fn reader (&mut self) -> Result<JsReadStream> {
        let read = self.get_read().await?;
        return JsReadStream::new(read.stream());
    }

    /// Returns a [`FileWrite`] that can be used to write contents to the file
    #[inline]
    pub async fn writer (&mut self) -> Result<FileWrite> {
        let (read, stream) = futures::try_join! {
            JsFuture::from(self.inner.get_file()),
            JsFuture::from(self.inner.create_writable())
        }?;
        
        return Ok(FileWrite {
            file: read.unchecked_into(),
            pos: 0,
            inner: JsWriteStream::new(stream.unchecked_into::<FileSystemWritableFileStream>())?
        });
    }

    #[inline]
    async fn get_read (&mut self) -> Result<JsFile> {
        let read = JsFuture::from(self.inner.get_file())
            .await?
            .unchecked_into::<JsFile>();

        return Ok(read)
    }

    /// Returns `true` if the specified permissions are granted for this specific file, and `false` otherwise.
    /// 
    /// # Compatibility
    /// Check the [compatibility table](https://developer.mozilla.org/en-US/docs/Web/API/FileSystemHandle/requestPermission#browser_compatibility)
    pub async fn get_permission (&self, write: bool) -> Result<bool> {
        let mut ops = FileSystemHandlePermissionDescriptor::new();
        ops.write(write)?;
    
        loop {
            let perm = JsFuture::from(self.inner.request_permission(&ops)?).await?;
            match PermisionStatus::from_js_value(&perm) {
                Some(PermisionStatus::Denied) => return Ok(false),
                Some(PermisionStatus::Granted) => return Ok(true),
                _ => {}
            }
        }
    }
}

/// A [`File`]'s metadata
#[derive(Clone)]
pub struct Metadata {
    len: u64,
    last_modified: i32
}

impl Metadata {
    /// Returns the size (in bytes) of the file
    #[inline]
    pub fn len (&self) -> u64 {
        return self.len
    }

    /// Returns the number of milliseconds that have passed from the UNIX EPOCH until the last time the file was modified.
    /// Check the [Compatibility Table]()
    /// 
    /// # Warning
    /// The Chromium implementation of this method in JavaScript is currently broken, and will not return a meaningful result
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

// File writting stream
pub struct FileWrite {
    file: JsFile,
    pos: u64,
    inner: JsWriteStream
}

impl FileWrite {
    #[inline]
    pub fn into_write_stream (self) -> JsWriteStream {
        return self.inner
    }

    #[inline]
    pub async fn write_chunk (&mut self, buf: &[u8]) -> Result<()> {
        self.inner.write_chunk(buf).await?;
        self.pos += buf.len() as u64;
        return Ok(())
    }

    pub async fn seek (&mut self, pos: SeekFrom) -> Result<()> {
        let offset = match pos {
            SeekFrom::Start(offset) => offset,

            SeekFrom::End(offset) => match (self.file.size() as u64).checked_add_signed(offset) {
                Some(x) => x,
                None => return Err(JsValue::from_str("arithmetic overflow"))
            },

            SeekFrom::Current(offset) => match self.pos.checked_add_signed(offset) {
                Some(x) => x,
                None => return Err(JsValue::from_str("arithmetic overflow"))
            }
        };

        JsFuture::from(self.inner.stream.unchecked_ref::<FileSystemWritableFileStream>().seek(offset)).await?;
        self.pos = offset;
        return Ok(())
    }
}

#[derive(Debug, Clone, PartialEq)]
#[repr(transparent)]
pub struct FileSystemHandlePermissionDescriptor (js_sys::Object);

impl FileSystemHandlePermissionDescriptor {
    #[inline]
    pub fn new () -> Self {
        return Self (js_sys::Object::new())
    }

    #[inline]
    pub fn write (&mut self, write: bool) -> Result<&mut Self> {
        js_sys::Reflect::set(
            &self.0,
            &JsValue::from_str("mode"),
            &if write { JsValue::from_str("readwrite") } else { JsValue::from_str("read") }
        )?;

        return Ok(self)
    }
}

impl Deref for FileSystemHandlePermissionDescriptor {
    type Target = js_sys::Object;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}