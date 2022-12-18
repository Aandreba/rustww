use std::{time::{Duration}};
use into_string::IntoString;
use web_sys::{NotificationOptions, NotificationPermission};
use crate::{Result};

pub(crate) enum Delay {
    Duration (Duration),
    Date (chrono::DateTime<chrono::Utc>)
}

/// Notification builder.
pub struct Notification {
    pub(crate) title: String,
    pub(crate) body: Option<String>,
    pub(crate) open: Option<Delay>,
    pub(crate) close: Option<Delay>
}

impl Notification {
    /// Creates a new notification builder
    #[inline]
    pub fn new (title: impl IntoString) -> Self {
        Self {
            title: title.into_string(),
            body: None,
            open: None,
            close: None
        }
    }

    /// Appends a new body to the notification. By default, notifiactions don't have a body.
    #[inline]
    pub fn body (mut self, body: impl IntoString) -> Self {
        self.body = Some(body.into_string());
        self   
    }

    /// Makes the notification fire with the specified delay after [`spawn`](Notification::spawn) is called.
    /// By default, notifiactions don't have a delay.
    #[inline]
    pub fn fire_after (mut self, delay: Duration) -> Self {
        self.open = Some(Delay::Duration(delay));
        self
    }

    /// Sets a new date for the notification to fire at.
    /// By default, notifiactions don't have a delay.
    pub fn fire_date<Tz: chrono::TimeZone> (mut self, date: chrono::DateTime<Tz>) -> Self {
        self.open = Some(Delay::Date(date.with_timezone(&chrono::Utc)));
        self
    }

    /// Makes the notification close with the specified delay after it's fired.
    /// By default, notifiactions don't close automatically.
    #[inline]
    pub fn close_after (mut self, delay: Duration) -> Self {
        self.close = Some(Delay::Duration(delay));
        self
    }

    /// Makes the notification close with the specified delay after it's fired.
    /// By default, notifiactions don't close automatically.
    pub fn close_date<Tz: chrono::TimeZone> (mut self, date: chrono::DateTime<Tz>) -> Self {
        self.close = Some(Delay::Date(date.with_timezone(&chrono::Utc)));
        self
    }

    /// Spawns a [`Future`](std::future::Future) that will wait for the specified fire delay, show the notification, and wait the specified close delay before closing it.
    /// 
    /// # Panics
    /// The spawned future will panic if the user doesn't grant permission to show notifications.
    pub fn spawn (self) {
        async fn wait_delay (delay: Option<Delay>) -> Result<()> {
            if let Some(delay) = delay {
                let delay = match delay {
                    Delay::Duration(dur) => dur,
                    Delay::Date(date) => (date - chrono::Utc::now()).to_std().unwrap()
                };
                crate::time::sleep(delay)?.await
            }
            
            return Ok(())
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

/// Returns the notification permissions granted by the user. If the user hasn't specified them yet,
/// [`request_permission`](web_sys::Notification::request_permission) will be called
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