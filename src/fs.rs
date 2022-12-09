use std::{time::{SystemTime, Duration}, fmt::Debug, ops::{Deref, RangeInclusive}, io::SeekFrom};
use chrono::{DateTime, Utc, NaiveDateTime};
use js_sys::{Array};
use wasm_bindgen::{prelude::wasm_bindgen, JsCast, JsValue};
use wasm_bindgen_futures::JsFuture;
use web_sys::{Window, HtmlInputElement};
use crate::{Result, window, io::{JsReadStream, JsWriteStream}};

type JsFile = web_sys::File;

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
    #[derive(Debug, Clone, PartialEq)]
    type FileSystemHandle;
    #[wasm_bindgen(extends = FileSystemHandle)]
    #[derive(Debug, Clone, PartialEq)]
    type FileSystemFileHandle;
    #[wasm_bindgen(extends = web_sys::WritableStream)]
    #[derive(Debug, Clone, PartialEq)]
    type FileSystemWritableFileStream;

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

#[derive(Debug, Clone, PartialEq)]
enum FileInner {
    Handle (FileSystemFileHandle),
    File (JsFile)
}

/// Object that provides acces to a file on the filesystem.
#[derive(Debug)]
pub struct File {
    inner: FileInner
}

impl File {
    /// Creates new instances of [`File`] from the files returned by the file selector.
    /// 
    /// # Compatibility
    /// Check the [compatibility table](https://developer.mozilla.org/en-US/docs/Web/API/window/showOpenFilePicker#browser_compatibility)
    pub async fn from_picker () -> Result<impl Iterator<Item = File>> {
        let picker = JsFuture::from(show_open_file_picker(&window()?)).await?;
        let array = picker.unchecked_into::<Array>();

        let iter = (0..array.length()).into_iter()
            .map(move |i| File {
                inner: FileInner::Handle(array.get(i).unchecked_into::<FileSystemFileHandle>())
            });

        return Ok(iter)
    }

    /// Creates new instances of [`File`] for each file selected in the `<input type="file" />`.
    /// 
    /// If the input isn't `type="file"` or hasn't selected any file, an empty iterator will be returned.
    pub fn from_input (input: &HtmlInputElement) -> impl Iterator<Item = File> {
        return input.files()
            .into_iter()
            .flat_map(|files| {
                (0..files.length()).map(move |i| File {
                    inner: FileInner::File(files.get(i).unwrap())
                })
            })
    }

    /// Returns the file's metadata
    #[inline]
    pub async fn metadata (&mut self) -> Result<Metadata> {
        let read = self.get_read().await?;
        return Ok(Metadata { len: read.size() as u64, last_modified: read.last_modified() })
    }

    /// Returns a [`JsReadStream`] that can be used to read the contents of the file
    #[inline]
    pub async fn reader (&mut self) -> Result<JsReadStream<'static>> {
        let read = self.get_read().await?;
        return JsReadStream::new(read.stream());
    }

    /// Returns a [`FileWrite`] that can be used to write contents to the file.
    /// 
    /// If [`File`] was created via [`from_input`](File::from_input), the result will always be `Ok(None)`
    /// 
    /// # Compatibility
    /// For instances created via [`from_picker`](File::from_picker), check the [compatibility table](https://developer.mozilla.org/en-US/docs/Web/API/FileSystemWritableFileStream#browser_compatibility).
    #[inline]
    pub async fn writer (&mut self) -> Result<Option<FileWrite>> {
        if let FileInner::Handle(ref mut inner) = self.inner {
            let (read, stream) = futures::try_join! {
                JsFuture::from(inner.get_file()),
                JsFuture::from(inner.create_writable())
            }?;
            
            return Ok(Some(FileWrite {
                file: read.unchecked_into(),
                pos: 0,
                inner: JsWriteStream::new(stream.unchecked_into::<FileSystemWritableFileStream>())?
            }));
        }

        return Ok(None)
    }

    #[inline]
    async fn get_read (&mut self) -> Result<JsFile> {
        match &mut self.inner {
            FileInner::File(file) => return Ok(file.clone()),
            FileInner::Handle(inner) => {
                let read = JsFuture::from(inner.get_file())
                    .await?
                    .unchecked_into::<JsFile>();

                return Ok(read)
            }
        }
    }

    /// Returns `true` if the specified permissions are granted for this specific file, and `false` otherwise.
    /// 
    /// If [`File`] was created via [`from_input`](File::from_input), the result will always be `Ok(false)`
    /// 
    /// # Compatibility
    /// For instances created via [`from_picker`](File::from_picker), check the [compatibility table](https://developer.mozilla.org/en-US/docs/Web/API/FileSystemHandle/requestPermission#browser_compatibility)
    pub async fn get_permission (&self, write: bool) -> Result<bool> {
        match &self.inner {
            FileInner::File(_) => return Ok(false),
            FileInner::Handle(inner) => {
                let mut ops = FileSystemHandlePermissionDescriptor::new();
                ops.write(write)?;
            
                loop {
                    let perm = JsFuture::from(inner.request_permission(&ops)?).await?;
                    match PermisionStatus::from_js_value(&perm) {
                        Some(PermisionStatus::Denied) => return Ok(false),
                        Some(PermisionStatus::Granted) => return Ok(true),
                        _ => {}
                    }
                }
            }
        }
    }
}

/// A [`File`]'s metadata
#[derive(Debug, Clone)]
pub struct Metadata {
    len: u64,
    last_modified: f64
}

impl Metadata {
    /// Returns the size (in bytes) of the file
    #[inline]
    pub fn len (&self) -> u64 {
        return self.len
    }

    /// Returns the number of milliseconds that have passed from the UNIX EPOCH until the last time the file was modified.
    /// 
    /// # Warning
    /// The Chromium implementation of this method in JavaScript is currently broken, and will not return a meaningful value
    #[inline]
    pub fn modified_millis (&self) -> f64 {
        return self.last_modified
    }

    /// Returns the last time the file was modified in [`SystemTime`]
    /// 
    /// # Warning
    /// The Chromium implementation of this method in JavaScript is currently broken, and will not return a meaningful value
    #[inline]
    pub fn modified (&self) -> Result<SystemTime> {
        let time = match self.last_modified {
            x if x.is_sign_negative() => Duration::try_from_secs_f64(-1000f64 * x).ok().and_then(|x| SystemTime::UNIX_EPOCH.checked_sub(x)),
            x => Duration::try_from_secs_f64(1000f64 * x).ok().and_then(|x| SystemTime::UNIX_EPOCH.checked_add(x))
        };
        return time.ok_or_else(|| JsValue::from_str("unsupported timestamp"))
    }

    /// Returns the last time the file was modified in a JavaScript [`Date`](js_sys::Date)
    /// 
    /// # Warning
    /// The Chromium implementation of this method in JavaScript is currently broken, and will not return a meaningful value
    #[inline]
    pub fn modified_date_js (&self) -> js_sys::Date {
        return js_sys::Date::new(&JsValue::from_f64(self.last_modified))
    }

    /// Returns the last time the file was modified in a Rust [`DateTime<Utc>`]
    /// 
    /// # Warning
    /// The Chromium implementation of this method in JavaScript is currently broken, and will not return a meaningful value
    #[inline]
    pub fn modified_date (&self) -> Result<DateTime<Utc>> {
        const RANGE: RangeInclusive<f64> = RangeInclusive::new(i64::MIN as f64, i64::MAX as f64);
        
        if RANGE.contains(&self.last_modified) {
            let naive = NaiveDateTime::from_timestamp_millis(self.last_modified as i64)
                .ok_or_else(|| JsValue::from_str("unsupported timestamp"))?;
            return Ok(DateTime::from_utc(naive, Utc));
        }

        return Err(JsValue::from_str("unsupported timestamp"))
    }
}

/// A writer to a [`File`] instance
pub struct FileWrite {
    file: JsFile,
    pos: u64,
    inner: JsWriteStream
}

impl FileWrite {
    /// Converts [`FileWrite`] into [`JsWriteStream`]
    #[inline]
    pub fn into_write_stream (self) -> JsWriteStream {
        return self.inner
    }

    /// Writes a chunk into the file instance
    #[inline]
    pub async fn write_chunk (&mut self, buf: &[u8]) -> Result<()> {
        self.inner.write_chunk(buf).await?;
        self.pos += buf.len() as u64;
        return Ok(())
    }

    /// Moves the file cursor to the specified position
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

/// Permission options 
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