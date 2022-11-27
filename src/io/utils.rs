use fast_async_trait::{async_trait_def, async_trait_impl};
use wasm_bindgen::JsValue;
use crate::Result;

#[async_trait_def]
pub trait AsyncRead {
    async fn read<'a> (&'a mut self, buf: &'a mut [u8]) -> Result<usize>;

    async fn read_exact<'a> (&'a mut self, mut buf: &'a mut [u8]) -> Result<()> {
        while buf.len() > 0 {
            let len = self.read(buf).await?;
            buf = &mut buf[len..];
        }
        return Ok::<_, JsValue>(())
    }
}

#[async_trait_impl]
impl<T: ?Sized + futures::AsyncRead + Unpin> AsyncRead for T {
    #[inline]
    async fn read<'a> (&'a mut self, buf: &'a mut [u8]) -> Result<usize> {
        return match futures::AsyncReadExt::read(self, buf).await {
            Ok(x) => Ok(x),
            Err(e) => Err(JsValue::from_str(&e.to_string()))
        }
    }
}

#[async_trait_def]
pub trait AsyncWrite {
    async fn write<'a> (&'a mut self, buf: &'a [u8]) -> Result<usize>;

    #[inline]
    async fn write_all<'a> (&'a mut self, mut buf: &'a [u8]) -> Result<()> {
        while buf.len() > 0 {
            let len = self.write(buf).await?;
            buf = &buf[len..];
        }
        return Ok::<_, JsValue>(())
    }
}

#[async_trait_impl]
impl<T: ?Sized + futures::AsyncWrite + Unpin> AsyncWrite for T {
    #[inline]
    async fn write<'a> (&'a mut self, buf: &'a [u8]) -> Result<usize> {
        return match <Self as futures::AsyncWriteExt>::write(self, buf).await {
            Ok(x) => Ok(x),
            Err(e) => Err(JsValue::from_str(&e.to_string()))
        }
    }
}