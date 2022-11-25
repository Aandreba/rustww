use std::time::Duration;
use futures::Future;
use wasm_bindgen::{prelude::wasm_bindgen, JsValue, JsCast};
use web_sys::BatteryManager;
use crate::{Result};
use wasm_bindgen::closure::Closure;
use std::collections::VecDeque;
use wasm_bindgen::__rt::WasmRefCell;
use std::rc::Rc;
use std::task::*;
use futures::Stream;

#[wasm_bindgen]
extern {
    #[wasm_bindgen(catch, js_namespace = navigator, js_name = getBattery)]
    async fn get_battery (this: &web_sys::Navigator) -> Result<JsValue>;
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BatteryTime {
    /// Amount of time that remain until the battery is fully charged or 0 if the battery is already fully charged.
    Charging (Duration),
    /// Amount of time that remains until the battery is fully discharged.
    Discharging (Duration)
}

#[derive(Clone)]
pub struct Battery {
    inner: web_sys::BatteryManager
}

impl Battery {
    pub async fn new () -> Result<Self> {
        let nav = web_sys::window().unwrap().navigator();
        let value = get_battery(&nav).await?;

        let inner: web_sys::BatteryManager;
        cfg_if::cfg_if! {
            if #[cfg(debug_assertions)] {
                inner = value.dyn_into().unwrap();
            } else {
                inner = value.unchecked_into();
            }
        }

        return Ok(Self { inner })
    }

    #[inline]
    pub fn new_snapshot () -> impl Future<Output = Result<Snapshot>> {
        return Snapshot::new()
    }

    #[inline]
    pub fn snapshot (&self) -> Snapshot {
        return Snapshot::from(&self.inner)
    }

    #[inline]
    pub fn charging (&self) -> bool {
        self.inner.charging()
    }

    #[inline]
    pub fn battery_time (&self) -> Option<BatteryTime> {
        return BatteryTime::try_from(&self.inner).ok()
    }

    #[inline]
    pub fn level (&self) -> f64 {
        self.inner.level()
    }
}

macro_rules! impl_watch {
    ($($jsname:literal as $fn:ident: $watch:ident => $name:ident: $ty:ty),+
    ) => {
        impl Battery {
            $(
                pub fn $fn (&self) -> Result<$watch> {
                    let buffer = Rc::new(WasmRefCell::new((VecDeque::new(), None::<Waker>)));
            
                    let my_self = self.clone();
                    let my_buffer = buffer.clone();
                    let resolve = Closure::<dyn FnMut(web_sys::Event)>::new(move |_: web_sys::Event| {
                        let mut my_buffer = my_buffer.borrow_mut();
                        my_buffer.0.push_back(my_self.$name());
                        if let Some(waker) = my_buffer.1.take() { waker.wake() }
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
                    win.add_event_listener_with_callback_and_bool($jsname, listener, true)?;
            
                    return Ok($watch {
                        resolve,
                        buffer
                    })
                }
            )+
        }

        $(
            pub struct $watch {
                #[allow(unused)]
                resolve: Closure<dyn FnMut(web_sys::Event)>,
                buffer: Rc<WasmRefCell<(VecDeque<$ty>, Option<Waker>)>>,
            }
            
            impl Stream for $watch {
                type Item = $ty;
            
                fn poll_next(self: std::pin::Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> Poll<Option<Self::Item>> {
                    let mut buffer = self.buffer.borrow_mut();
                    if let Some(geo) = buffer.0.pop_front() {
                        return Poll::Ready(Some(geo))
                    }
            
                    buffer.1 = Some(cx.waker().clone());
                    return Poll::Pending
                }
            }
            
            impl Drop for $watch {
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
                    win.remove_event_listener_with_callback_and_bool($jsname, listener, true).unwrap();
                }
            }
        )+
    };
}

impl_watch! {
    "levelchange" as watch_level: LevelWatcher => level: f64,
    "chargingchange" as watch_charging: ChargingWatcher => charging: bool
}

impl Battery {
    pub fn battery_time_watcher (&self) -> Result<BatteryTimeWatcher> {
        let buffer = Rc::new(WasmRefCell::new((VecDeque::new(), None::<Waker>)));
            
        let my_self = self.inner.clone();
        let my_buffer = buffer.clone();
        let charge = Closure::<dyn FnMut(web_sys::Event)>::new(move |_: web_sys::Event| {
            let value = my_self.charging_time();
            if value == f64::INFINITY { return }
            
            let mut my_buffer = my_buffer.borrow_mut();
            my_buffer.0.push_back(BatteryTime::Charging(Duration::from_secs_f64(value)));
            if let Some(waker) = my_buffer.1.take() { waker.wake() }
        });

        let my_self = self.inner.clone();
        let my_buffer = buffer.clone();
        let discharge = Closure::<dyn FnMut(web_sys::Event)>::new(move |_: web_sys::Event| {
            let value = my_self.charging_time();
            if value == f64::INFINITY { return }
            
            let mut my_buffer = my_buffer.borrow_mut();
            my_buffer.0.push_back(BatteryTime::Discharging(Duration::from_secs_f64(value)));
            if let Some(waker) = my_buffer.1.take() { waker.wake() }
        });

        let charge_listener: &js_sys::Function;
        let discharge_listener: &js_sys::Function;
        cfg_if::cfg_if! {
            if #[cfg(debug_assertions)] {
                charge_listener = charge.as_ref().dyn_ref().unwrap();
                discharge_listener = discharge.as_ref().dyn_ref().unwrap();
            } else {
                charge_listener = charge.as_ref().unchecked_ref();
                discharge_listener = discharge.as_ref().unchecked_ref();
            }
        }

        let win = web_sys::window().unwrap();
        win.add_event_listener_with_callback_and_bool("chargingtimechange", charge_listener, true)?;
        win.add_event_listener_with_callback_and_bool("dischargingtimechange", discharge_listener, true)?;

        return Ok(BatteryTimeWatcher {
            charge,
            discharge,
            buffer
        })
    }
}

pub struct BatteryTimeWatcher {
    #[allow(unused)]
    charge: Closure<dyn FnMut(web_sys::Event)>,
    #[allow(unused)]
    discharge: Closure<dyn FnMut(web_sys::Event)>,
    buffer: Rc<WasmRefCell<(VecDeque<BatteryTime>, Option<Waker>)>>,
}

impl Stream for BatteryTimeWatcher {
    type Item = BatteryTime;

    fn poll_next(self: std::pin::Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> Poll<Option<Self::Item>> {
        let mut buffer = self.buffer.borrow_mut();
        if let Some(geo) = buffer.0.pop_front() {
            return Poll::Ready(Some(geo))
        }

        buffer.1 = Some(cx.waker().clone());
        return Poll::Pending
    }
}

impl Drop for BatteryTimeWatcher {
    fn drop(&mut self) {
        let charge: &js_sys::Function;
        let discharge: &js_sys::Function;

        cfg_if::cfg_if! {
            if #[cfg(debug_assertions)] {
                charge = self.charge.as_ref().dyn_ref().unwrap();
                discharge = self.discharge.as_ref().dyn_ref().unwrap();
            } else {
                charge = self.charge.as_ref().unchecked_ref();
                discharge = self.discharge.as_ref().unchecked_ref();
            }
        }

        let win = web_sys::window().unwrap();
        win.remove_event_listener_with_callback_and_bool("chargingtimechange", charge, true).unwrap();
        win.remove_event_listener_with_callback_and_bool("dischargingtimechange", discharge, true).unwrap();
    }
}

#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct Snapshot {
    /// Indicates whether or not the device's battery is currently being charged.
    pub charging: bool,
    pub battery_time: Option<BatteryTime>,
    /// Indicates the current battery charge level as a value between 0.0 and 1.0.
    /// - A value of 0.0 means the battery is empty and the system is about to be suspended.
    /// - A value of 1.0 means the battery is full.
    /// - A value of 1.0 is also returned if the implementation isn't able to determine the battery charge level or if the system is not battery-powered.
    pub level: f64
}

impl Snapshot {
    #[inline]
    pub async fn new () -> Result<Self> {
        let nav = web_sys::window().unwrap().navigator();
        let value = get_battery(&nav).await?;

        let manager: web_sys::BatteryManager;
        cfg_if::cfg_if! {
            if #[cfg(debug_assertions)] {
                manager = value.dyn_into().unwrap();
            } else {
                manager = value.unchecked_into();
            }
        }

        return Ok(Self::from(manager))
    }
}

impl From<&BatteryManager> for Snapshot {
    #[inline]
    fn from(value: &BatteryManager) -> Self {
        return Self {
            charging: value.charging(),
            battery_time: BatteryTime::try_from(value).ok(),
            level: value.level()
        }
    }
}

impl TryFrom<&BatteryManager> for BatteryTime {
    type Error = (f64, f64);

    #[inline]
    fn try_from(value: &BatteryManager) -> ::core::result::Result<Self, Self::Error> {
        let charge = value.charging_time();
        if charge == f64::INFINITY {
            let discharge = value.discharging_time();
            if discharge == f64::INFINITY {
                return Err((charge, discharge))
            }
            return Ok(Self::Discharging(Duration::from_secs_f64(discharge)))
        }
        return Ok(Self::Charging(Duration::from_secs_f64(charge)))
    }
}

impl From<BatteryManager> for Snapshot {
    #[inline]
    fn from(value: BatteryManager) -> Self {
        return Self::from(&value)
    }
}

impl TryFrom<BatteryManager> for BatteryTime {
    type Error = (f64, f64);

    #[inline]
    fn try_from(value: BatteryManager) -> ::core::result::Result<Self, Self::Error> {
        Self::try_from(&value)
    }
}