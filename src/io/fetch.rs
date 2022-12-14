use std::{rc::Rc, sync::Arc, fmt::Debug};
use js_sys::Uint8Array;
use serde::{de::DeserializeOwned};
use wasm_bindgen::{JsValue, prelude::wasm_bindgen, JsCast};
use wasm_bindgen_futures::JsFuture;
use web_sys::{RequestInit, RequestCache, RequestCredentials, Headers, RequestMode, RequestRedirect, ReferrerPolicy};
use crate::{Result, utils::{AbortController, AbortSignal}, scope::fetch};
use super::{JsReadStream};

macro_rules! impl_ident {
    ($($t:ty),+) => {
        $(
            impl IntoFetchBody for $t {
                #[inline]
                fn into_body (self) -> Option<JsValue> {
                    Some(self.into())
                }
            }

            impl IntoFetchBody for &$t {
                #[inline]
                fn into_body (self) -> Option<JsValue> {
                    Some(self.into())
                }
            }
        )+
    };
}

/// Represents an element that can be used as a [`Request`] body.
pub trait IntoFetchBody {
    fn into_body (self) -> Option<JsValue>;
}

impl_ident! {
    web_sys::Blob,
    js_sys::ArrayBuffer,
    js_sys::Uint8Array,
    js_sys::Uint8ClampedArray,
    js_sys::Uint16Array,
    js_sys::Uint32Array,
    js_sys::BigUint64Array,
    js_sys::Int8Array,
    js_sys::Int16Array,
    js_sys::Int32Array,
    js_sys::BigInt64Array,
    js_sys::DataView,
    web_sys::FormData,
    web_sys::UrlSearchParams,
    js_sys::JsString,
    web_sys::ReadableStream
}

impl<T: IntoFetchBody> IntoFetchBody for Option<T> {
    #[inline]
    fn into_body (self) -> Option<JsValue> {
        self.and_then(IntoFetchBody::into_body)
    }
}

impl IntoFetchBody for &str {
    #[inline]
    fn into_body (self) -> Option<JsValue> {
        return Some(JsValue::from_str(self))
    }
}

impl IntoFetchBody for String {
    #[inline]
    fn into_body (self) -> Option<JsValue> {
        return Some(JsValue::from_str(&self))
    }
}

impl IntoFetchBody for Box<str> {
    #[inline]
    fn into_body (self) -> Option<JsValue> {
        return Some(JsValue::from_str(&self))
    }
}

impl IntoFetchBody for Rc<str> {
    #[inline]
    fn into_body (self) -> Option<JsValue> {
        return Some(JsValue::from_str(&self))
    }
}

impl IntoFetchBody for Arc<str> {
    #[inline]
    fn into_body (self) -> Option<JsValue> {
        return Some(JsValue::from_str(&self))
    }
}

/// A fetch request's method
#[derive(Clone, Copy, PartialEq, Eq, Hash, Default)]
#[wasm_bindgen]
pub enum Method {
    #[default]
    Get = "GET",
    Post = "POST",
    Head = "HEAD",
    Put = "PUT"
}

/// A builder that allows to customize the parameters for an HTTP request
#[derive(Default)]
pub struct Request {
    inner: RequestInit,
    headers: Option<Headers>
}

impl Request {
    /// Creates a new request
    #[inline]
    pub fn new () -> Self {
        return Default::default()
    }

    /// Executes an HTTP GET request with the default parameters, targeting the specified url
    #[inline]
    pub async fn get (url: &str) -> Result<Response> {
        return Self::new().fetch(url).await
    }

    /// Assigns a body to the request
    #[inline]
    pub fn body (&mut self, body: impl IntoFetchBody) -> &mut Self {
        self.inner.body(body.into_body().as_ref());
        self
    }

    /// Specifies the cache mode of the request
    #[inline]
    pub fn cache (&mut self, cache: RequestCache) -> &mut Self {
        self.inner.cache(cache);
        self
    }

    /// Indicates whether the user agent should send or receive cookies from the other domain in the case of cross-origin requests.
    #[inline]
    pub fn credentials (&mut self, credentials: RequestCredentials) -> &mut Self {
        self.inner.credentials(credentials);
        self
    }

    /// Adds the specified header to the request
    #[inline]
    pub fn header (&mut self, key: &str, value: &str) -> Result<&mut Self> {
        if self.headers.is_none() {
            self.headers = Some(Headers::new()?);
        }

        let headers = unsafe { self.headers.as_ref().unwrap_unchecked() };
        headers.set(key, value)?;
        return Ok(self)
    }

    /// Adds the specified headers to the request
    #[inline]
    pub fn headers<K: AsRef<str>, V: AsRef<str>> (&mut self, headers: impl IntoIterator<Item = (K, V)>) -> Result<&mut Self> {
        for (key, value) in headers {
            self.header(key.as_ref(), value.as_ref())?;
        }
        return Ok(self)
    }

    /// Sets the [subresource integrity](https://developer.mozilla.org/en-US/docs/Web/Security/Subresource_Integrity) value of the request.
    #[inline]
    pub fn integrity (&mut self, integrity: &str) -> &mut Self {
        self.inner.integrity(integrity);
        self
    }

    /// Specifies the HTTP method the request will be sent as.
    #[inline]
    pub fn method (&mut self, method: Method) -> &mut Self {
        self.inner.method(method.to_str());
        self
    }

    /// Specifies the mode of the request
    #[inline]
    pub fn mode (&mut self, mode: RequestMode) -> &mut Self {
        self.inner.mode(mode);
        self
    }

    /// Indicates how to handle redirects of the request
    #[inline]
    pub fn redirect (&mut self, redirect: RequestRedirect) -> &mut Self {
        self.inner.redirect(redirect);
        self
    }

    /// Sets the referrer of the request
    #[inline]
    pub fn referrer (&mut self, referrer: &str) -> &mut Self {
        self.inner.referrer(referrer);
        self
    }

    /// Assigns the referrer policy, which governs what referrer information (sent in the Referer header) should be included with the request. 
    #[inline]
    pub fn referrer_policy (&mut self, referrer_policy: ReferrerPolicy) -> &mut Self {
        self.inner.referrer_policy(referrer_policy);
        self
    }

    /// Makes the request abortable, returning it's [`AbortController`]
    #[inline]
    pub fn abortable<T> (&mut self) -> Result<(AbortController<T>, &mut Self)> {
        let con = AbortController::new()?;
        let _ = self.abortable_with(&con);
        return Ok((con, self))
    }

    /// Assigns `con` as the abort controller of the request
    #[inline]
    pub fn abortable_with<T> (&mut self, con: &AbortController<T>) -> &mut Self {
        self.abortable_with_raw(&con.raw_signal())
    }

    /// Assigns `signal` as the signal to abort the request
    #[inline]
    pub fn abortable_with_signal<T> (&mut self, signal: &AbortSignal<T>) -> &mut Self {
        self.abortable_with_raw(signal.as_ref())
    }

    /// Assigns `signal` as the signal to abort the request
    #[inline]
    pub fn abortable_with_raw (&mut self, signal: &web_sys::AbortSignal) -> &mut Self {
        self.inner.signal(Some(signal));
        self
    }

    /// Executes the request, returning it's [`Response`]
    #[inline]
    pub async fn fetch (mut self, url: &str) -> Result<Response> {
        if let Some(headers) = self.headers {
            self.inner.headers(&headers);
        }

        let req = web_sys::Request::new_with_str_and_init(url, &self.inner)?;
        let fetch = JsFuture::from(fetch(&req)).await?;
        debug_assert!(fetch.is_instance_of::<web_sys::Response>());

        return Ok(Response {
            inner: fetch.unchecked_into()
        })
    }
}

/// Reponse to a HTTP [`Request`]
pub struct Response {
    inner: web_sys::Response
}

impl Response {
    /// Returns the body of the reponse as a [`JsReadStream`], if available.
    /// Otherwise, `None` is returned.
    #[inline]
    pub fn body (self) -> Result<Option<JsReadStream<'static, Uint8Array>>> {
        return self.inner.body().map(JsReadStream::new).transpose()
    }

    /// Attempts to convert the [`Response`] into it's body, returning itself as an error
    /// if it doesn't have one.
    #[inline]
    pub fn try_body (self) -> Result<::core::result::Result<JsReadStream<'static, Uint8Array>, Self>> {
        return match self.inner.body() {
            Some(x) => JsReadStream::new(x).map(Ok),
            None => Ok(Err(self))
        }
    }

    /// Returns the URL of the response
    #[inline]
    pub fn url (&self) -> String {
        return self.inner.url()
    }

    /// Returns the status code of the response
    #[inline]
    pub fn status (&self) -> u16 {
        return self.inner.status()
    }

    /// Returns `true` if the response is successful, `false` otherwise.
    /// 
    /// A response is considered successful when it's status code is in the 200-299 range
    #[inline]
    pub fn ok (&self) -> bool {
        return self.inner.ok()
    }

    /// Returns `true` if the response is the result of a redirected request.
    #[inline]
    pub fn redirected (&self) -> bool {
        self.inner.redirected()
    }

    /// Returns the response's body as a byte sequence
    pub async fn bytes (self) -> Result<Vec<u8>> {
        return match self.try_body()? {
            Ok(mut body) => body.read_remaining_bytes().await,
            Err(this) => {
                let value = JsFuture::from(this.inner.array_buffer()?).await?;
                let buffer = value.unchecked_into::<js_sys::ArrayBuffer>();
                let bytes = Uint8Array::new_with_byte_offset_and_length(&buffer, 0, buffer.byte_length());
                Ok(bytes.to_vec())
            }
        }
    }

    /// Returns the response's body as a UTF-8 parsed string
    pub async fn text (self) -> Result<String> {
        return match self.try_body()? {
            Ok(mut body) => {
                let bytes = body.read_remaining_bytes().await?;
                match String::from_utf8(bytes) {
                    Ok(string) => Ok(string),
                    Err(e) => Err(JsValue::from_str(&e.to_string()))
                }
            },

            Err(this) => {
                let text = JsFuture::from(this.inner.text()?).await?;
                debug_assert!(text.is_instance_of::<js_sys::JsString>());
                let text = text.unchecked_into::<js_sys::JsString>();
                Ok(ToString::to_string(&text))
            }
        }
    }

    /// Deserializes the response's body from JSON into the specified type.
    pub async fn json<T: DeserializeOwned> (self) -> Result<T> {
        return match self.try_body()? {
            Ok(mut body) => {
                let bytes = body.read_remaining_bytes().await?;
                match serde_json::from_slice::<T>(&bytes) {
                    Ok(json) => Ok(json),
                    Err(e) => Err(JsValue::from_str(&e.to_string()))
                }
            },

            Err(this) => {
                let json = JsFuture::from(this.inner.json()?).await?;
                let v = serde_wasm_bindgen::from_value::<T>(json)?;
                Ok(v)
            }
        }
    }
}

impl Debug for Response {
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        return write!(f, "{} {}", self.inner.status(), self.inner.status_text())
    }
}

impl Clone for Response {
    #[inline]
    fn clone(&self) -> Self {
        Self { inner: self.inner.clone().unwrap() }
    }
}