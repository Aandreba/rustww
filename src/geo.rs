use std::{task::{Poll}, future::Future};
use futures::{Stream, FutureExt};
use wasm_bindgen::{prelude::{wasm_bindgen, Closure}, JsCast};
use crate::{Result, utils::{LocalReceiver, local_channel}, scope::window, sync::{one_shot, ShotReceiver}};
use futures::StreamExt;

#[wasm_bindgen]
extern {
    type GeolocationPosition;
    type GeolocationCoordinates;

    #[wasm_bindgen(method, getter)]
    pub fn coords(this: &GeolocationPosition) -> GeolocationCoordinates;
    #[wasm_bindgen(method, getter)]
    fn timestamp(this: &GeolocationPosition) -> f64;

    #[wasm_bindgen(method, getter)]
    pub fn latitude(this: &GeolocationCoordinates) -> f64;
    #[wasm_bindgen(method, getter)]
    pub fn longitude(this: &GeolocationCoordinates) -> f64;
    #[wasm_bindgen(method, getter)]
    pub fn altitude(this: &GeolocationCoordinates) -> Option<f64>;
    #[wasm_bindgen(method, getter)]
    pub fn accuracy(this: &GeolocationCoordinates) -> f64;
    #[wasm_bindgen(method, getter, js_name = altitudeAccuracy)]
    pub fn altitude_accuracy(this: &GeolocationCoordinates) -> Option<f64>;
    #[wasm_bindgen(method, getter)]
    pub fn heading(this: &GeolocationCoordinates) -> Option<f64>;
    #[wasm_bindgen(method, getter)]
    pub fn speed(this: &GeolocationCoordinates) -> Option<f64>;
}

/// Information abut a specific geolocation
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct Geolocation {
    /// Represents the position's latitude in decimal degrees.
    pub latitude: f64,
    /// Represents the position's longitude in decimal degrees.
    pub longitude: f64,
    /// Represents the position's altitude in meters, relative to sea level. This value can be null if the implementation cannot provide the data.
    pub altitude: Option<f64>,
    /// Represents the accuracy of the latitude and longitude properties, expressed in meters.
    pub accuracy: f64,
    /// Represents the accuracy of the altitude expressed in meters. This value can be null.
    pub altitude_accuracy: Option<f64>,
    /// Represents the direction towards which the device is facing. This value, specified in degrees, indicates how far off from heading true north the device is. 0 degrees represents true north, and the direction is determined clockwise (which means that east is 90 degrees and west is 270 degrees). If speed is 0, heading is NaN. If the device is unable to provide heading information, this value is null.
    pub heading: Option<f64>,
    /// Represents the velocity of the device in meters per second. This value can be null.
    pub speed: Option<f64>
}

impl Geolocation {
    /// Returns a [`Future`] that resolves to the current geolocation of the device
    pub fn current () -> Result<CurrentGeolocation> {
        let (send, inner) = one_shot();
        let resolve_closure = Closure::once(move |loc: GeolocationPosition| {
            let _ = send.try_send(loc);
        });

        let resolve: &js_sys::Function;
        cfg_if::cfg_if! {
            if #[cfg(debug_assertions)] {
                resolve = resolve_closure.as_ref().dyn_ref().unwrap();
            } else {
                resolve = resolve_closure.as_ref().unchecked_ref();
            }
        }

        let geo = window()?.navigator().geolocation()?;
        match geo.get_current_position(resolve) {
            Ok(_) => {
                resolve_closure.forget();
            },
            Err(e) => return Err(e)
        }

        return Ok(CurrentGeolocation { inner })
    }

    /// Returns a watcher for the device's geolocation
    #[inline]
    pub fn watch () -> Result<GeolocationWatcher> {
        return GeolocationWatcher::new()
    }
}

/// A watcher for a device's [`Geolocation`].
/// 
/// Every time the geolocation of the device changes, [`GeolocationWatcher`] will be notified.
/// 
/// When droped, the watcher will be closed, releasing all the memory of it's closure, avoiding a memory leak.
pub struct GeolocationWatcher {
    id: i32,
    _success: Closure<dyn FnMut(GeolocationPosition)>,
    recv: LocalReceiver<Geolocation>
}

impl GeolocationWatcher {
    /// Creates a new [`GeolocationWatcher`]
    #[inline]
    pub fn new () -> Result<Self> {
        let (send, recv) = local_channel();
        let success = Closure::<dyn FnMut(GeolocationPosition)>::new(move |loc: GeolocationPosition| {
            let _ = send.try_send(Geolocation::from(loc));
        });

        let resolve: &js_sys::Function;
        cfg_if::cfg_if! {
            if #[cfg(debug_assertions)] {
                resolve = success.as_ref().dyn_ref().unwrap();
            } else {
                resolve = success.as_ref().unchecked_ref();
            }
        }

        let geo = window()?.navigator().geolocation()?;
        let id = geo.watch_position(resolve)?;
        return Ok(Self {
            id,
            _success: success,
            recv,
        })
    }
}

impl Stream for GeolocationWatcher {
    type Item = Geolocation;

    #[inline]
    fn poll_next(mut self: std::pin::Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> Poll<Option<Self::Item>> {
        self.recv.poll_next_unpin(cx)
    }
}

impl Drop for GeolocationWatcher {
    #[inline]
    fn drop(&mut self) {
        let geo = window().unwrap().navigator().geolocation().unwrap();
        geo.clear_watch(self.id);
    }
}

impl From<&GeolocationCoordinates> for Geolocation {
    fn from(coords: &GeolocationCoordinates) -> Self {
        return Self {
            latitude: coords.latitude(),
            longitude: coords.longitude(),
            accuracy: coords.accuracy(),
            altitude: coords.altitude(),
            altitude_accuracy: coords.altitude_accuracy(),
            heading: coords.heading(),
            speed: coords.speed()
        }
    }
}

impl From<&GeolocationPosition> for Geolocation {
    #[inline]
    fn from(value: &GeolocationPosition) -> Self {
        let coords = value.coords();
        return Self::from(coords)
    }
}

impl From<GeolocationCoordinates> for Geolocation {
    #[inline]
    fn from(value: GeolocationCoordinates) -> Self {
        Geolocation::from(&value)
    }
}

impl From<GeolocationPosition> for Geolocation {
    #[inline]
    fn from(value: GeolocationPosition) -> Self {
        Geolocation::from(&value)
    }
}

/// Future for [`current`](Geolocation::current)
pub struct CurrentGeolocation {
    inner: ShotReceiver<GeolocationPosition>
}

impl Future for CurrentGeolocation {
    type Output = Geolocation;

    #[inline]
    fn poll(mut self: std::pin::Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> std::task::Poll<Self::Output> {
        return match self.inner.poll_unpin(cx) {
            Poll::Ready(Some(x)) => Poll::Ready(Geolocation::from(x)),
            Poll::Ready(None) => unreachable!(),
            Poll::Pending => Poll::Pending
        }
    }
}