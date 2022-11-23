use wasm_bindgen::{prelude::wasm_bindgen, JsValue, JsCast};
use web_sys::BatteryManager;
use crate::{Result};

#[wasm_bindgen]
extern {
    #[wasm_bindgen(catch, js_name = getBattery)]
    async fn get_battery (this: &web_sys::Navigator) -> Result<JsValue>;
}

pub enum BatteryTime {
    /// Amount of time, in seconds, that remain until the battery is fully charged or 0 if the battery is already fully charged.
    Charging (f64),
    /// Amount of time, in seconds, that remains until the battery is fully discharged.
    Discharging (f64)
}

pub struct Battery {
    
}

pub struct Snapshot {
    /// Indicates whether or not the device's battery is currently being charged.
    pub charging: bool,
    pub battery_time: Option<BatteryTime>,
    /// Indicates the current battery charge level as a value between 0.0 and 1.0.
    /// A value of 0.0 means the battery is empty and the system is about to be suspended.
    /// A value of 1.0 means the battery is full.
    /// A value of 1.0 is also returned if the implementation isn't able to determine the battery charge level or if the system is not battery-powered.
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
            return Ok(Self::Discharging(discharge))
        }

        return Ok(Self::Charging(charge))
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