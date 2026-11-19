//! Operating system battery & AC power policy inspection for zero-waste background sync.

use gitupload_core::models::PowerState;

/// Queries OS power metrics to guard background synchronization during low battery or Battery Saver mode.
pub struct PowerMonitor;

impl PowerMonitor {
    /// Reads the live OS power state (`AC`, `BatteryNormal`, or `BatteryLowOrSaver`).
    pub fn get_current_power_state() -> PowerState {
        #[cfg(windows)]
        {
            use windows_sys::Win32::System::Power::{GetSystemPowerStatus, SYSTEM_POWER_STATUS};

            let mut status = SYSTEM_POWER_STATUS {
                ACLineStatus: 0,
                BatteryFlag: 0,
                BatteryLifePercent: 255,
                SystemStatusFlag: 0,
                BatteryLifeTime: 0,
                BatteryFullLifeTime: 0,
            };

            // SAFETY: Calling Win32 `GetSystemPowerStatus` with a valid, initialized mutable struct pointer.
            #[allow(unsafe_code)]
            let ok = unsafe { GetSystemPowerStatus(&mut status) };
            if ok != 0 {
                if status.ACLineStatus == 1 {
                    return PowerState::AcConnected;
                }
                let pct_low = status.BatteryLifePercent != 255 && status.BatteryLifePercent <= 20;
                let flag_low = (status.BatteryFlag & 2) != 0 || (status.BatteryFlag & 4) != 0;
                let saver_on = status.SystemStatusFlag == 1;
                if pct_low || flag_low || saver_on {
                    return PowerState::BatteryLowOrSaver;
                }
                return PowerState::BatteryNormal;
            }
        }

        PowerState::AcConnected
    }

    /// Evaluates whether scheduled background sync cycles may execute given user battery preferences.
    pub fn should_allow_background_sync(allow_on_battery: bool) -> bool {
        Self::evaluate_policy(Self::get_current_power_state(), allow_on_battery)
    }

    /// Pure policy evaluator testable independently of hardware state.
    pub fn evaluate_policy(state: PowerState, allow_on_battery: bool) -> bool {
        match state {
            PowerState::AcConnected => true,
            PowerState::BatteryNormal => allow_on_battery,
            PowerState::BatteryLowOrSaver => false,
        }
    }
}
