use std::time::Duration;
use futures::Future;
use wasm_bindgen::{prelude::wasm_bindgen, JsValue, JsCast};
use web_sys::BatteryManager;
use crate::{Result, window};
use wasm_bindgen::closure::Closure;
use std::task::*;
use futures::Stream;
use futures::StreamExt;
use crate::utils::*;

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
        let nav = window()?.navigator();
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
                    let (send, recv) = local_channel();
                    let resolve = Closure::<dyn FnMut(web_sys::Event)>::new(move |evt: web_sys::Event| {
                        let my_self = evt.current_target().unwrap();
                        debug_assert!(my_self.is_instance_of::<BatteryManager>());
                        let _ = send.try_send(my_self.unchecked_into::<BatteryManager>().$name());
                    });
            
                    let listener: &js_sys::Function;
                    cfg_if::cfg_if! {
                        if #[cfg(debug_assertions)] {
                            listener = resolve.as_ref().dyn_ref().unwrap();
                        } else {
                            listener = resolve.as_ref().unchecked_ref();
                        }
                    }
            
                    self.inner.add_event_listener_with_callback($jsname, listener)?;
            
                    return Ok($watch {
                        inner: self.inner.clone(),
                        resolve,
                        recv
                    })
                }
            )+
        }

        $(
            pub struct $watch {
                inner: web_sys::BatteryManager,
                #[allow(unused)]
                resolve: Closure<dyn FnMut(web_sys::Event)>,
                recv: LocalReceiver<$ty>
            }
            
            impl Stream for $watch {
                type Item = $ty;
            
                #[inline]
                fn poll_next(mut self: std::pin::Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> Poll<Option<Self::Item>> {
                    self.recv.poll_next_unpin(cx)
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
            
                    self.inner.remove_event_listener_with_callback($jsname, listener).unwrap();
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
    pub fn watch_battery_time (&self) -> Result<BatteryTimeWatcher> {
        let (send, recv) = local_channel();

        let my_send = send.clone();
        let charge = Closure::<dyn FnMut(web_sys::Event)>::new(move |evt: web_sys::Event| {
            let my_self = evt.current_target().unwrap();
            debug_assert!(my_self.is_instance_of::<BatteryManager>());
            let my_self = my_self.unchecked_into::<BatteryManager>();

            let value = my_self.charging_time();
            if value == f64::INFINITY { return }
            let _ = my_send.try_send(BatteryTime::Charging(Duration::from_secs_f64(value)));
        });

        let discharge = Closure::<dyn FnMut(web_sys::Event)>::new(move |evt: web_sys::Event| {
            let my_self = evt.current_target().unwrap();
            debug_assert!(my_self.is_instance_of::<BatteryManager>());
            let my_self = my_self.unchecked_into::<BatteryManager>();

            let value = my_self.discharging_time();
            if value == f64::INFINITY { return }
            let _ = send.try_send(BatteryTime::Discharging(Duration::from_secs_f64(value)));
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

        self.inner.add_event_listener_with_callback("chargingtimechange", charge_listener)?;
        self.inner.add_event_listener_with_callback("dischargingtimechange", discharge_listener)?;

        return Ok(BatteryTimeWatcher {
            inner: self.inner.clone(),
            charge,
            discharge,
            recv
        })
    }
}

pub struct BatteryTimeWatcher {
    inner: BatteryManager,
    #[allow(unused)]
    charge: Closure<dyn FnMut(web_sys::Event)>,
    #[allow(unused)]
    discharge: Closure<dyn FnMut(web_sys::Event)>,
    recv: LocalReceiver<BatteryTime>
}

impl Stream for BatteryTimeWatcher {
    type Item = BatteryTime;

    #[inline]
    fn poll_next(mut self: std::pin::Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> Poll<Option<Self::Item>> {
        self.recv.poll_next_unpin(cx)
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

        self.inner.remove_event_listener_with_callback("chargingtimechange", charge).unwrap();
        self.inner.remove_event_listener_with_callback("dischargingtimechange", discharge).unwrap();
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
        let nav = window()?.navigator();
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