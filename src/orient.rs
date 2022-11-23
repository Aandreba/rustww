use std::{rc::Rc, task::{Waker, Poll}, collections::VecDeque};
use futures::{Stream};
use wasm_bindgen::{prelude::Closure, __rt::WasmRefCell};
use web_sys::{DeviceOrientationEvent, DeviceMotionEvent, DeviceAcceleration, DeviceRotationRate};
use crate::{Result, utils::OneShot, math::Vec3d};
use wasm_bindgen::JsCast;

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct EulerAngles {
    /// Represents the motion of the device around the z axis, represented in degrees with values ranging from 0 (inclusive) to 360 (exclusive).
    pub alpha: f64,
    /// Represents the motion of the device around the x axis, represented in degrees with values ranging from -180 (inclusive) to 180 (exclusive). This represents a front to back motion of the device.
    pub beta: f64,
    /// Represents the motion of the device around the y axis, represented in degrees with values ranging from -90 (inclusive) to 90 (exclusive). This represents a left to right motion of the device.
    pub gamma: f64
}

impl EulerAngles {
    #[inline]
    pub fn to_vec (self) -> Vec3d {
        return Vec3d::new(self.beta, self.gamma, self.alpha)
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
#[non_exhaustive]
pub struct Orientation {
    /// Indicates whether or not the device is providing orientation data absolutely (that is, in reference to the Earth's coordinate frame) or using some arbitrary frame determined by the device.
    pub absolute: bool,
    pub angles: EulerAngles
}

impl Orientation {
    pub async fn current () -> Result<Self> {
        let (result, send) = OneShot::new();
        let f = Closure::<dyn FnMut(DeviceOrientationEvent)>::new(move |evt: DeviceOrientationEvent| {
            send.try_send(Orientation::from(evt)).unwrap();
        });

        let listener: &js_sys::Function;
        cfg_if::cfg_if! {
            if #[cfg(debug_assertions)] {
                listener = f.as_ref().dyn_ref().unwrap();
            } else {
                listener = f.as_ref().unchecked_ref();
            }
        }
        
        let win = web_sys::window().unwrap();
        win.add_event_listener_with_callback_and_bool("deviceorientation", listener, true)?;
        let result = result.await;
        win.remove_event_listener_with_callback_and_bool("deviceorientation", listener, true)?;
        return Ok(result);
    }

    #[inline]
    pub fn watch () -> Result<OrientationWatcher> {
        return OrientationWatcher::new()
    }

    #[docfg::docfg(target_feature = "atomics")]
    #[inline]
    pub fn watch_send () -> Result<SendOrientationWatcher> {
        return SendOrientationWatcher::new()
    }
}

pub struct OrientationWatcher {
    #[allow(unused)]
    resolve: Closure<dyn FnMut(DeviceOrientationEvent)>,
    buffer: Rc<WasmRefCell<(VecDeque<Orientation>, Vec<Waker>)>>,
}

impl OrientationWatcher {
    #[inline]
    pub fn new () -> Result<Self> {
        let buffer = Rc::new(WasmRefCell::new((VecDeque::new(), Vec::new())));

        let my_buffer = buffer.clone();
        let resolve = Closure::<dyn FnMut(DeviceOrientationEvent)>::new(move |evt: DeviceOrientationEvent| {
            let geo = Orientation::from(evt);
            let mut my_buffer = my_buffer.borrow_mut();
            my_buffer.0.push_back(geo);
            my_buffer.1.drain(..).for_each(Waker::wake);
        });

        let listener: &js_sys::Function;
        cfg_if::cfg_if! {
            if #[cfg(debug_assertions)] {
                listener = resolve.as_ref().dyn_ref().unwrap();
            } else {
                listener = resolve.as_ref().unchecked_ref();
            }
        }

        let win = web_sys::window().unwrap();
        win.add_event_listener_with_callback_and_bool("deviceorientation", listener, true)?;

        return Ok(Self {
            resolve,
            buffer,
        })
    }
}

impl Stream for OrientationWatcher {
    type Item = Orientation;

    fn poll_next(self: std::pin::Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> Poll<Option<Self::Item>> {
        let mut buffer = self.buffer.borrow_mut();
        if let Some(geo) = buffer.0.pop_front() {
            return Poll::Ready(Some(geo))
        }

        buffer.1.push(cx.waker().clone());
        return Poll::Pending
    }
}

impl Drop for OrientationWatcher {
    fn drop(&mut self) {
        let listener: &js_sys::Function;
        cfg_if::cfg_if! {
            if #[cfg(debug_assertions)] {
                listener = self.resolve.as_ref().dyn_ref().unwrap();
            } else {
                listener = self.resolve.as_ref().unchecked_ref();
            }
        }

        let win = web_sys::window().unwrap();
        win.remove_event_listener_with_callback_and_bool("deviceorientation", listener, true).unwrap();
    }
}

cfg_if::cfg_if! {
    if #[cfg(any(test, target_feature = "atomics"))] {
        use futures::StreamExt;
        use crate::send::{SyncableClosure, syncable_wrapped_closure};

        #[cfg_attr(docsrs, doc(cfg(target_feature = "atomics")))]
        pub struct SendOrientationWatcher {
            #[allow(unused)]
            resolve: SyncableClosure<dyn Fn(DeviceOrientationEvent) + Send + Sync>,
            recv: async_channel::Receiver<Orientation>
        }
        
        impl SendOrientationWatcher {
            #[inline]
            pub fn new () -> Result<Self> {
                let (send, recv) = async_channel::unbounded();
                let closure = Box::new(move |evt: DeviceOrientationEvent| {
                    let send = send.clone();
                    let fut = async move { let _ = send.send(Orientation::from(evt)).await; };
                    wasm_bindgen_futures::spawn_local(fut);
                });

                let win = web_sys::window().unwrap();
                let listener = unsafe { syncable_wrapped_closure::<dyn Fn(DeviceOrientationEvent), _>(&closure) };
                win.add_event_listener_with_callback_and_bool("deviceorientation", &listener, true)?;
        
                return Ok(Self {
                    resolve: SyncableClosure::new(listener.sync(), closure),
                    recv,
                })
            }
        }
        
        impl Stream for SendOrientationWatcher {
            type Item = Orientation;
        
            #[inline]
            fn poll_next(mut self: std::pin::Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> Poll<Option<Self::Item>> {
                self.recv.poll_next_unpin(cx)
            }
        }
        
        impl Drop for SendOrientationWatcher {
            #[inline]
            fn drop(&mut self) {        
                let win = web_sys::window().unwrap();
                unsafe {
                    win.remove_event_listener_with_callback_and_bool("deviceorientation", self.resolve.function(), true).unwrap()
                };
            }
        }        
    }
}

#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct Motion {
    /// Amount of acceleration recorded by the device, in meters per second squared (m/s²).
    /// The acceleration value does not include the effect of the gravity force, in contrast to [`acceleration_with_gravity`].
    pub acceleration: Vec3d,
    /// Amount of acceleration recorded by the device, in meters per second squared (m/s²).
    /// Unlike [`acceleration`] which compensates for the influence of gravity,
    /// its value is the sum of the acceleration of the device as induced by the user and an acceleration equal and opposite to that caused by gravity.
    /// In other words, it measures the g-force. In practice, this value represents the raw data measured by an accelerometer.
    pub acceleration_with_gravity: Vec3d,
    /// Interval, in milliseconds, at which data is obtained from the underlying hardware.
    pub interval: f64,
    /// Rate at which the device is rotating around each of its axes in degrees per second.
    pub rotation_rate: Option<EulerAngles>
}

impl Motion {
    pub async fn current () -> Result<Self> {
        let (result, send) = OneShot::new();
        let f = Closure::<dyn FnMut(DeviceMotionEvent)>::new(move |evt: DeviceMotionEvent| {
            send.try_send(Motion::from(evt)).unwrap();
        });

        let listener: &js_sys::Function;
        cfg_if::cfg_if! {
            if #[cfg(debug_assertions)] {
                listener = f.as_ref().dyn_ref().unwrap();
            } else {
                listener = f.as_ref().unchecked_ref();
            }
        }
        
        let win = web_sys::window().unwrap();
        win.add_event_listener_with_callback_and_bool("devicemotion", listener, true)?;
        let result = result.await;
        win.remove_event_listener_with_callback_and_bool("devicemotion", listener, true)?;

        return Ok(result);
    }

    #[inline]
    pub fn watch () -> Result<MotionWatcher> {
        return MotionWatcher::new()
    }
}

pub struct MotionWatcher {
    #[allow(unused)]
    resolve: Closure<dyn FnMut(DeviceMotionEvent)>,
    buffer: Rc<WasmRefCell<(VecDeque<Motion>, Vec<Waker>)>>,
}

impl MotionWatcher {
    #[inline]
    pub fn new () -> Result<Self> {
        let buffer = Rc::new(WasmRefCell::new((VecDeque::new(), Vec::new())));

        let my_buffer = buffer.clone();
        let resolve = Closure::<dyn FnMut(DeviceMotionEvent)>::new(move |evt: DeviceMotionEvent| {
            let geo = Motion::from(evt);
            let mut my_buffer = my_buffer.borrow_mut();
            my_buffer.0.push_back(geo);
            my_buffer.1.drain(..).for_each(Waker::wake);
        });

        let listener: &js_sys::Function;
        cfg_if::cfg_if! {
            if #[cfg(debug_assertions)] {
                listener = resolve.as_ref().dyn_ref().unwrap();
            } else {
                listener = resolve.as_ref().unchecked_ref();
            }
        }

        let win = web_sys::window().unwrap();
        win.add_event_listener_with_callback_and_bool("devicemotion", listener, true)?;

        return Ok(Self {
            resolve,
            buffer,
        })
    }
}

impl Stream for MotionWatcher {
    type Item = Motion;

    fn poll_next(self: std::pin::Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> Poll<Option<Self::Item>> {
        let mut buffer = self.buffer.borrow_mut();
        if let Some(geo) = buffer.0.pop_front() {
            return Poll::Ready(Some(geo))
        }

        buffer.1.push(cx.waker().clone());
        return Poll::Pending
    }
}

impl Drop for MotionWatcher {
    fn drop(&mut self) {
        let listener: &js_sys::Function;
        cfg_if::cfg_if! {
            if #[cfg(debug_assertions)] {
                listener = self.resolve.as_ref().dyn_ref().unwrap();
            } else {
                listener = self.resolve.as_ref().unchecked_ref();
            }
        }

        let win = web_sys::window().unwrap();
        win.remove_event_listener_with_callback_and_bool("devicemotion", listener, true).unwrap();
    }
}

/*cfg_if::cfg_if! {
    if #[cfg(target_feature = "atomics")] {
        pub struct SendMotionWatcher {
            #[allow(unused)]
            resolve: Closure<dyn Send + FnMut(DeviceMotionEvent)>,
            recv: async_channel::Receiver<Motion>
        }
        
        impl MotionWatcher {
            #[inline]
            pub fn new () -> Result<Self> {
                let buffer = Rc::new(WasmRefCell::new((VecDeque::new(), Vec::new())));
        
                let my_buffer = buffer.clone();
                let resolve = Closure::<dyn FnMut(DeviceMotionEvent)>::new(move |evt: DeviceMotionEvent| {
                    let geo = Motion::from(evt);
                    let mut my_buffer = my_buffer.borrow_mut();
                    my_buffer.0.push_back(geo);
                    my_buffer.1.drain(..).for_each(Waker::wake);
                });
        
                let listener: &js_sys::Function;
                cfg_if::cfg_if! {
                    if #[cfg(debug_assertions)] {
                        listener = resolve.as_ref().dyn_ref().unwrap();
                    } else {
                        listener = resolve.as_ref().unchecked_ref();
                    }
                }
        
                let win = web_sys::window().unwrap();
                win.add_event_listener_with_callback_and_bool("devicemotion", listener, true)?;
        
                return Ok(Self {
                    resolve,
                    buffer,
                })
            }
        }
        
        impl Stream for MotionWatcher {
            type Item = Motion;
        
            fn poll_next(self: std::pin::Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> Poll<Option<Self::Item>> {
                let mut buffer = self.buffer.borrow_mut();
                if let Some(geo) = buffer.0.pop_front() {
                    return Poll::Ready(Some(geo))
                }
        
                buffer.1.push(cx.waker().clone());
                return Poll::Pending
            }
        }
        
        impl Drop for MotionWatcher {
            fn drop(&mut self) {
                let listener: &js_sys::Function;
                cfg_if::cfg_if! {
                    if #[cfg(debug_assertions)] {
                        listener = self.resolve.as_ref().dyn_ref().unwrap();
                    } else {
                        listener = self.resolve.as_ref().unchecked_ref();
                    }
                }
        
                let win = web_sys::window().unwrap();
                win.remove_event_listener_with_callback_and_bool("devicemotion", listener, true).unwrap();
            }
        }
    }
}*/

impl From<&DeviceMotionEvent> for Motion {
    #[inline]
    fn from(value: &DeviceMotionEvent) -> Self {
        return Self {
            acceleration: value.acceleration().unwrap().into(),
            acceleration_with_gravity: value.acceleration_including_gravity().unwrap().into(),
            interval: value.interval().unwrap(),
            rotation_rate: value.rotation_rate().map(EulerAngles::from)
        }
    }
}

impl From<&DeviceAcceleration> for Vec3d {
    #[inline]
    fn from(value: &DeviceAcceleration) -> Self {
        return Self::new(value.x().unwrap(), value.y().unwrap(), value.z().unwrap())
    }
}

impl From<&DeviceOrientationEvent> for Orientation {
    #[inline]
    fn from(value: &DeviceOrientationEvent) -> Self {
        Self {
            absolute: value.absolute(),
            angles: value.into()
        }
    }
}

impl From<&DeviceOrientationEvent> for EulerAngles {
    #[inline]
    fn from(value: &DeviceOrientationEvent) -> Self {
        Self {
            alpha: value.alpha().unwrap(),
            beta: value.beta().unwrap(),
            gamma: value.gamma().unwrap()
        }
    }
}

impl From<&DeviceRotationRate> for EulerAngles {
    #[inline]
    fn from(value: &DeviceRotationRate) -> Self {
        Self {
            alpha: value.alpha().unwrap(),
            beta: value.beta().unwrap(),
            gamma: value.gamma().unwrap()
        }
    }
}

impl From<DeviceMotionEvent> for Motion {
    #[inline]
    fn from(value: DeviceMotionEvent) -> Self {
        return Self::from(&value)
    }
}

impl From<DeviceAcceleration> for Vec3d {
    #[inline]
    fn from(value: DeviceAcceleration) -> Self {
        return Self::from(&value)
    }
}

impl From<DeviceOrientationEvent> for Orientation {
    #[inline]
    fn from(value: DeviceOrientationEvent) -> Self {
        return Orientation::from(&value)
    }
}

impl From<DeviceOrientationEvent> for EulerAngles {
    #[inline]
    fn from(value: DeviceOrientationEvent) -> Self {
        return EulerAngles::from(&value)
    }
}

impl From<DeviceRotationRate> for EulerAngles {
    #[inline]
    fn from(value: DeviceRotationRate) -> Self {
        return Self::from(&value)
    }
}