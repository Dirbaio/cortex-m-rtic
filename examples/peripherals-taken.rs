#![deny(unsafe_code)]
#![deny(warnings)]
#![no_main]
#![no_std]

use cortex_m_semihosting::debug;
use panic_semihosting as _;

#[rtic::app(device = lm3s6965)]
mod app {
    #[init]
    fn init(_: init::Context) -> init::LateResources {
        assert!(cortex_m::Peripherals::take().is_none());
        debug::exit(debug::EXIT_SUCCESS);

        init::LateResources {}
    }
}
