use std::{rc::Rc, sync::Arc, fmt::Debug};
use futures::{StreamExt, TryStreamExt};
use js_sys::Uint8Array;
use serde::{de::DeserializeOwned};
use wasm_bindgen::{JsValue, prelude::wasm_bindgen, JsCast};
use wasm_bindgen_futures::JsFuture;
use web_sys::{RequestInit, RequestCache, RequestCredentials, Headers, RequestMode, RequestRedirect, ReferrerPolicy};
use crate::{Result, window};
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
#[derive(Default)]
#[wasm_bindgen]
pub enum Method {
    #[default]
    Get = "GET",
    Post = "POST",
    Head = "HEAD"
}

#[derive(Default)]
pub struct Request {
    inner: RequestInit,
    headers: Option<Headers>
}

impl Request {
    #[inline]
    pub fn new () -> Self {
        return Default::default()
    }

    #[inline]
    pub async fn get (url: &str) -> Result<Response> {
        return Self::new().fetch(url).await
    }

    #[inline]
    pub fn body (&mut self, body: impl IntoFetchBody) -> &mut Self {
        self.inner.body(body.into_body().as_ref());
        self
    }

    #[inline]
    pub fn cache (&mut self, cache: RequestCache) -> &mut Self {
        self.inner.cache(cache);
        self
    }

    #[inline]
    pub fn credentials (&mut self, credentials: RequestCredentials) -> &mut Self {
        self.inner.credentials(credentials);
        self
    }

    #[inline]
    pub fn header (&mut self, key: &str, value: &str) -> Result<&mut Self> {
        if self.headers.is_none() {
            self.headers = Some(Headers::new()?);
        }

        let headers = unsafe { self.headers.as_ref().unwrap_unchecked() };
        headers.set(key, value)?;
        return Ok(self)
    }

    #[inline]
    pub fn headers<K: AsRef<str>, V: AsRef<str>> (&mut self, headers: impl IntoIterator<Item = (K, V)>) -> Result<&mut Self> {
        for (key, value) in headers {
            self.header(key.as_ref(), value.as_ref())?;
        }
        return Ok(self)
    }

    #[inline]
    pub fn integrity (&mut self, integrity: &str) -> &mut Self {
        self.inner.integrity(integrity);
        self
    }

    #[inline]
    pub fn method (&mut self, method: Method) -> &mut Self {
        self.inner.method(method.to_str());
        self
    }

    #[inline]
    pub fn mode (&mut self, mode: RequestMode) -> &mut Self {
        self.inner.mode(mode);
        self
    }

    #[inline]
    pub fn redirect (&mut self, redirect: RequestRedirect) -> &mut Self {
        self.inner.redirect(redirect);
        self
    }

    #[inline]
    pub fn referrer (&mut self, referrer: &str) -> &mut Self {
        self.inner.referrer(referrer);
        self
    }

    #[inline]
    pub fn referrer_policy (&mut self, referrer_policy: ReferrerPolicy) -> &mut Self {
        self.inner.referrer_policy(referrer_policy);
        self
    }

    // todo signal

    #[inline]
    pub async fn fetch (mut self, url: &str) -> Result<Response> {
        if let Some(headers) = self.headers {
            self.inner.headers(&headers);
        }

        let req = web_sys::Request::new_with_str_and_init(url, &self.inner)?;
        let fetch = JsFuture::from(window()?.fetch_with_request(&req)).await?;
        debug_assert!(fetch.is_instance_of::<web_sys::Response>());

        return Ok(Response {
            inner: fetch.unchecked_into()
        })
    }
}

pub struct Response {
    inner: web_sys::Response
}

impl Response {
    #[inline]
    pub fn body (self) -> Result<Option<JsReadStream>> {
        return self.inner.body().map(JsReadStream::new).transpose()
    }

    #[inline]
    pub fn try_body (self) -> Result<::core::result::Result<JsReadStream, Self>> {
        return match self.inner.body() {
            Some(x) => JsReadStream::new(x).map(Ok),
            None => Ok(Err(self))
        }
    }

    #[inline]
    pub fn url (&self) -> String {
        return self.inner.url()
    }

    #[inline]
    pub fn status (&self) -> u16 {
        return self.inner.status()
    }

    #[inline]
    pub fn ok (&self) -> bool {
        return self.inner.ok()
    }

    #[inline]
    pub fn redirected (&self) -> bool {
        self.inner.redirected()
    }

    pub async fn bytes (self) -> Result<Vec<u8>> {
        return match self.try_body()? {
            Ok(mut body) => body.read_remaining().await,
            Err(this) => {
                let value = JsFuture::from(this.inner.array_buffer()?).await?;
                let buffer = value.unchecked_into::<js_sys::ArrayBuffer>();
                let bytes = Uint8Array::new_with_byte_offset_and_length(&buffer, 0, buffer.byte_length());
                Ok(bytes.to_vec())
            }
        }
    }

    pub async fn text (self) -> Result<String> {
        return match self.try_body()? {
            Ok(mut body) => {
                let bytes = body.read_remaining().await?;
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

    pub async fn json<T: DeserializeOwned> (self) -> Result<T> {
        return match self.try_body()? {
            Ok(mut body) => {
                let bytes = body.read_remaining().await?;
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