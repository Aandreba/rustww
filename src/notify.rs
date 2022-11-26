use std::{time::{Duration}};
use into_string::IntoString;
use web_sys::{NotificationOptions, NotificationPermission};
use crate::{Result};

pub(crate) enum Delay {
    Duration (Duration),
    Date (chrono::DateTime<chrono::Utc>)
}

pub struct Notification {
    pub(crate) title: String,
    pub(crate) body: Option<String>,
    pub(crate) open: Option<Delay>,
    pub(crate) close: Option<Delay>
}

impl Notification {
    #[inline]
    pub fn new (title: impl IntoString) -> Self {
        Self {
            title: title.into_string(),
            body: None,
            open: None,
            close: None
        }
    }

    #[inline]
    pub fn body (mut self, body: impl IntoString) -> Self {
        self.body = Some(body.into_string());
        self   
    }

    #[inline]
    pub fn fire_after (mut self, delay: Duration) -> Self {
        self.open = Some(Delay::Duration(delay));
        self
    }

    pub fn fire_date<Tz: chrono::TimeZone> (mut self, date: chrono::DateTime<Tz>) -> Self {
        self.open = Some(Delay::Date(date.with_timezone(&chrono::Utc)));
        self
    }

    #[inline]
    pub fn close_after (mut self, delay: Duration) -> Self {
        self.close = Some(Delay::Duration(delay));
        self
    }

    pub fn close_date<Tz: chrono::TimeZone> (mut self, date: chrono::DateTime<Tz>) -> Self {
        self.close = Some(Delay::Date(date.with_timezone(&chrono::Utc)));
        self
    }

    pub fn spawn_local (self) {
        async fn wait_delay (delay: Option<Delay>) {
            if let Some(delay) = delay {
                let delay = match delay {
                    Delay::Duration(dur) => dur,
                    Delay::Date(date) => (date - chrono::Utc::now()).to_std().unwrap()
                };

                if let Err(e) = crate::time::sleep(delay).await {
                    wasm_bindgen::throw_val(e)
                }
            }
        }

        wasm_bindgen_futures::spawn_local(async move {
            if !get_permision().await.is_ok_and(core::convert::identity) {
                panic!("Notification access denied");
            }
            
            let mut options = NotificationOptions::new();
            if let Some(body) = self.body {
                options.body(&body);
            }

            wait_delay(self.open).await;
            let notification: web_sys::Notification = web_sys::Notification::new_with_options(&self.title, &options).unwrap();
            wait_delay(self.close).await;
            notification.close();
        });
    }
}

pub async fn get_permision () -> Result<bool> {
    loop {
        match web_sys::Notification::permission() {
            NotificationPermission::Granted => return Ok(true),
            NotificationPermission::Denied => return Ok(false),
            NotificationPermission::Default => {
                wasm_bindgen_futures::JsFuture::from(
                    web_sys::Notification::request_permission()?
                ).await?;
            },
            _ => unreachable!()
        }
    }
}