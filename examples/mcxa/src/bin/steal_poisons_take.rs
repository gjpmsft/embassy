//! Demonstrates that `cortex_m::Peripherals::steal()` inside `embassy_mcxa::init()`
//! sets the internal `TAKEN` flag, causing any subsequent `cortex_m::Peripherals::take()`
//! to return `None`.
//!
//! This is a problem because HAL users reasonably expect to call `take()` after
//! `hal::init()` to obtain Cortex-M core peripherals (e.g. SCB for enabling fault
//! handlers). The `steal()` calls in `clocks::configure_voltages()` silently poison
//! the `take()` singleton.
//!
//! Expected: `cortex_m::Peripherals::take()` returns `Some(...)` after `hal::init()`.
//! Actual:   `cortex_m::Peripherals::take()` returns `None` after `hal::init()`.

#![no_std]
#![no_main]

use embassy_executor::Spawner;
use {defmt_rtt as _, embassy_mcxa as hal, panic_probe as _};

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    // hal::init() calls clocks::init() -> configure_voltages(), which internally
    // calls `cortex_m::Peripherals::steal()`. The steal() implementation in
    // cortex-m 0.7 unconditionally sets TAKEN = true.
    let _p = hal::init(hal::config::Config::default());

    // This will ALWAYS be None because steal() already set TAKEN = true.
    // Users who need SCB (e.g. to enable UsageFault/BusFault/MemManage handlers)
    // are forced to use steal() themselves, defeating the safety of take().
    match cortex_m::Peripherals::take() {
        Some(mut cp) => {
            defmt::info!("SUCCESS: cortex_m::Peripherals::take() returned Some");

            // Example: enable individual fault handlers instead of escalating to HardFault
            use cortex_m::peripheral::scb::Exception;
            cp.SCB.enable(Exception::MemoryManagement);
            cp.SCB.enable(Exception::BusFault);
            cp.SCB.enable(Exception::UsageFault);
            defmt::info!("Fault handlers enabled via take()");
        }
        None => {
            defmt::error!(
                "FAIL: cortex_m::Peripherals::take() returned None! \
                 steal() inside hal::init() poisoned the singleton."
            );

            // Workaround: forced to use steal(), which is unsafe and provides
            // no aliasing guarantees.
            let mut cp = unsafe { cortex_m::Peripherals::steal() };
            use cortex_m::peripheral::scb::Exception;
            cp.SCB.enable(Exception::MemoryManagement);
            cp.SCB.enable(Exception::BusFault);
            cp.SCB.enable(Exception::UsageFault);
            defmt::warn!("Fault handlers enabled via steal() workaround");
        }
    }

    loop {
        embassy_time::Timer::after_secs(1).await;
    }
}
