use std::{rc::Rc, task::{Waker, Poll}, future::Future, collections::VecDeque};
use futures::{Stream, TryFutureExt};
use wasm_bindgen::{prelude::{wasm_bindgen, Closure}, JsCast, JsValue, __rt::WasmRefCell};
use crate::{Result, utils::{OneShot}};

#[wasm_bindgen]
extern {
    pub type GeolocationPosition;
    pub type GeolocationCoordinates;

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

#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct Geolocation {
    /// Returns a double representing the position's latitude in decimal degrees.
    pub latitude: f64,
    /// Returns a double representing the position's longitude in decimal degrees.
    pub longitude: f64,
    /// Returns a double representing the position's altitude in meters, relative to sea level. This value can be null if the implementation cannot provide the data.
    pub altitude: Option<f64>,
    /// Returns a double representing the accuracy of the latitude and longitude properties, expressed in meters.
    pub accuracy: f64,
    /// Returns a double representing the accuracy of the altitude expressed in meters. This value can be null.
    pub altitude_accuracy: Option<f64>,
    /// Returns a double representing the direction towards which the device is facing. This value, specified in degrees, indicates how far off from heading true north the device is. 0 degrees represents true north, and the direction is determined clockwise (which means that east is 90 degrees and west is 270 degrees). If speed is 0, heading is NaN. If the device is unable to provide heading information, this value is null.
    pub heading: Option<f64>,
    /// Returns a double representing the velocity of the device in meters per second. This value can be null.
    pub speed: Option<f64>
}

impl Geolocation {
    pub fn current () -> Result<CurrentGeolocation> {
        let (inner, send) = OneShot::new();

        let my_result = send.clone();
        let resolve_closure = Closure::once(move |loc: GeolocationPosition| {
            let _ = my_result.try_send(Ok(loc));
        });

        let my_result = send.clone();
        let reject_closure = Closure::once(move |err: JsValue| {
            let _ = my_result.try_send(Err(err));
        });

        let resolve: &js_sys::Function;
        let reject: &js_sys::Function;
        cfg_if::cfg_if! {
            if #[cfg(debug_assertions)] {
                resolve = resolve_closure.as_ref().dyn_ref().unwrap();
                reject = reject_closure.as_ref().dyn_ref().unwrap();
            } else {
                resolve = resolve_closure.as_ref().unchecked_ref();
                reject = reject_closure.as_ref().unchecked_ref();
            }
        }

        let geo = web_sys::window().unwrap().navigator().geolocation()?;
        match geo.get_current_position_with_error_callback(resolve, Some(reject)) {
            Ok(_) => {
                resolve_closure.forget();
                reject_closure.forget();
            },
            Err(e) => return Err(e)
        }

        return Ok(CurrentGeolocation { inner })
    }

    #[inline]
    pub fn watch () -> Result<GeolocationWatcher> {
        return GeolocationWatcher::new()
    }

    #[docfg::docfg(target_feature = "atomics")]
    #[inline]
    pub fn watch_send () -> Result<SendGeolocationWatcher> {
        return SendGeolocationWatcher::new()
    }
}

pub struct GeolocationWatcher {
    id: i32,
    #[allow(unused)]
    success: Closure<dyn FnMut(GeolocationPosition)>,
    #[allow(unused)]
    failure: Closure<dyn FnMut(JsValue)>,
    buffer: Rc<WasmRefCell<(VecDeque<Result<Geolocation>>, Option<Waker>)>>,
}

impl GeolocationWatcher {
    #[inline]
    pub fn new () -> Result<Self> {
        let buffer = Rc::new(WasmRefCell::new((VecDeque::new(), None::<Waker>)));

        let my_buffer = buffer.clone();
        let success = Closure::<dyn FnMut(GeolocationPosition)>::new(move |loc: GeolocationPosition| {
            let geo = Geolocation::from(loc);
            let mut my_buffer = my_buffer.borrow_mut();
            my_buffer.0.push_back(Ok(geo));
            if let Some(waker) = my_buffer.1.take() { waker.wake() }
        });

        let my_buffer = buffer.clone();
        let failure = Closure::<dyn FnMut(JsValue)>::new(move |err: JsValue| {
            let mut my_buffer = my_buffer.borrow_mut();
            my_buffer.0.push_back(Err(err));
            if let Some(waker) = my_buffer.1.take() { waker.wake() }
        });

        let resolve: &js_sys::Function;
        let reject: &js_sys::Function;
        cfg_if::cfg_if! {
            if #[cfg(debug_assertions)] {
                resolve = success.as_ref().dyn_ref().unwrap();
                reject = failure.as_ref().dyn_ref().unwrap();
            } else {
                resolve = success.as_ref().unchecked_ref();
                reject = failure.as_ref().unchecked_ref();
            }
        }

        let geo = web_sys::window().unwrap().navigator().geolocation()?;
        let id = geo.watch_position_with_error_callback(resolve, Some(reject))?;
        return Ok(Self {
            id,
            success,
            failure,
            buffer,
        })
    }
}

impl Stream for GeolocationWatcher {
    type Item = Result<Geolocation>;

    fn poll_next(self: std::pin::Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> Poll<Option<Self::Item>> {
        let mut buffer = self.buffer.borrow_mut();
        if let Some(geo) = buffer.0.pop_front() {
            return Poll::Ready(Some(geo))
        }

        buffer.1 = Some(cx.waker().clone());
        return Poll::Pending
    }
}

impl Drop for GeolocationWatcher {
    #[inline]
    fn drop(&mut self) {
        let geo = web_sys::window().unwrap().navigator().geolocation().unwrap();
        geo.clear_watch(self.id);
    }
}

cfg_if::cfg_if! {
    if #[cfg(any(test, target_feature = "atomics"))] {
        use futures::StreamExt;
        use crate::send::{syncable_wrapped_closure, SyncableClosure};

        #[cfg_attr(docsrs, doc(cfg(target_feature = "atomics")))]
        pub struct SendGeolocationWatcher {
            id: i32,
            #[allow(unused)]
            success: SyncableClosure<dyn Fn(GeolocationPosition) + Send + Sync>,
            recv: async_channel::Receiver<Geolocation>
        }

        impl SendGeolocationWatcher {
            #[inline]
            pub fn new () -> Result<Self> {
                let (send, recv) = async_channel::unbounded();
                let closure = Box::new(move |loc: GeolocationPosition| {
                    let send = send.clone();
                    let fut = async move { let _ = send.send(Geolocation::from(loc)).await; };
                    wasm_bindgen_futures::spawn_local(fut);
                });

                let resolve = unsafe { syncable_wrapped_closure::<dyn Fn(GeolocationPosition), _>(&closure) };
                let geo = web_sys::window().unwrap().navigator().geolocation()?;
                let id = geo.watch_position(&resolve)?;

                return Ok(Self {
                    id,
                    success: SyncableClosure::new(resolve.sync(), closure),
                    recv,
                })
            }
        }
        
        impl Stream for SendGeolocationWatcher {
            type Item = Geolocation;
        
            #[inline]
            fn poll_next(mut self: std::pin::Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> Poll<Option<Self::Item>> {
                self.recv.poll_next_unpin(cx)
            }
        }
        
        impl Drop for SendGeolocationWatcher {
            #[inline]
            fn drop(&mut self) {
                let geo = web_sys::window().unwrap().navigator().geolocation().unwrap();
                geo.clear_watch(self.id);
            }
        }
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

pub struct CurrentGeolocation {
    inner: OneShot<Result<GeolocationPosition>>
}

impl Future for CurrentGeolocation {
    type Output = Result<Geolocation>;

    #[inline]
    fn poll(mut self: std::pin::Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> std::task::Poll<Self::Output> {
        if let Poll::Ready(x) = self.inner.try_poll_unpin(cx)? {
            return Poll::Ready(Ok(Geolocation::from(x)))
        }
        return Poll::Pending
    }
}