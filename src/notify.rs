use std::time::{Duration, Instant, SystemTime};
use into_string::IntoString;

enum Delay {
    Duration (Duration),
    Instant (Instant),
    Time (SystemTime),
    #[cfg(feature = "chrono")]
    Date (chrono::DateTime<Utc>)
}

pub struct Notification {
    title: String,
    body: Option<String>,
    delay: Option<Delay>
}

impl Notification {
    #[inline]
    pub fn new (title: impl IntoString) -> Self {
        Self {
            title: title.into_string(),
            body: None,
            delay: None
        }
    }

    #[inline]
    pub fn body (mut self, body: impl IntoString) -> Self {
        self.body = Some(body.into_string());
        self   
    }

    #[inline]
    pub fn fire_after (mut self, delay: Duration) -> Self {
        self.delay = Some(Delay::Duration(delay));
        self
    }

    #[inline]
    pub fn fire_instant (mut self, instant: Instant) -> Self {
        self.delay = Some(Delay::Instant(instant));
        self
    }

    #[inline]
    pub fn fire_time (mut self, time: SystemTime) -> Self {
        self.delay = Some(Delay::Time(time));
        self
    }

    #[cfg(feature = "chrono")]
    pub fn fire_date<Tz: chrono::TimeZone> (mut self, time: SystemTime) -> Self {
        self.delay = Some(Delay::Time(time));
        self
    }
}