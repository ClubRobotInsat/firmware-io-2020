use crate::f103;
use crate::f103::interrupt;
use crate::f103::Peripherals;
use crate::f103::SPI1;
use crate::hal::delay::Delay;
use crate::hal::gpio::{
    gpioa::*, gpiob::*, gpioc::*, Alternate, Floating, Input, Output, PushPull,
};
use crate::hal::prelude::*;

use crate::hal::device::Interrupt::TIM1_UP;
use crate::hal::device::TIM2;
use crate::hal::pwm::{Pwm, C1};
use crate::hal::spi::*;
use crate::hal::timer::Event;
use crate::hal::timer::Timer;
use crate::CortexPeripherals;
use cortex_m_rt::exception;
use cortex_m_rt::ExceptionFrame;
use embedded_hal::spi::FullDuplex;
use pwm_speaker::Speaker;
use stm32f1xx_hal::gpio::PullDown; //  Stack frame for exception handling.

pub type SpiPins = (
    PA5<Alternate<PushPull>>,
    PA6<Input<Floating>>,
    PA7<Alternate<PushPull>>,
);

pub struct Robot {
    pub delay: Delay,
    pub led_communication: PC14<Output<PushPull>>,
    pub pump: PA4<Output<PushPull>>,
    pub valves: [PBx<Output<PushPull>>; 4],
    pub tirette: PB1<Input<PullDown>>,
    pub speaker: Speaker,
}

pub fn init_peripherals(
    chip: Peripherals,
    mut cortex: CortexPeripherals,
) -> (Robot, Spi<SPI1, SpiPins>, PB13<Output<PushPull>>) {
    // Config des horloges
    let mut rcc = chip.RCC.constrain();
    let mut flash = chip.FLASH.constrain();
    let mut afio = chip.AFIO.constrain(&mut rcc.apb2);

    cortex.DCB.enable_trace();
    cortex.DWT.enable_cycle_counter();

    let clocks = rcc
        .cfgr
        .use_hse(8.mhz())
        .sysclk(72.mhz())
        .pclk1(36.mhz())
        .pclk2(72.mhz())
        .freeze(&mut flash.acr);

    //  Configuration des GPIOs
    let mut gpioa = chip.GPIOA.split(&mut rcc.apb2);
    let mut gpiob = chip.GPIOB.split(&mut rcc.apb2);
    let mut gpioc = chip.GPIOC.split(&mut rcc.apb2);

    // Configuration des PINS

    // Slave select, on le fixe à un état bas (on n'en a pas besoin, une seule communication)
    let mut cs = gpiob.pb13.into_push_pull_output(&mut gpiob.crh);
    cs.set_low();

    let sclk = gpioa.pa5.into_alternate_push_pull(&mut gpioa.crl);
    let miso = gpioa.pa6.into_floating_input(&mut gpioa.crl);
    let mosi = gpioa.pa7.into_alternate_push_pull(&mut gpioa.crl);

    let vannes = [
        gpiob.pb12.into_push_pull_output(&mut gpiob.crh).downgrade(),
        gpiob.pb14.into_push_pull_output(&mut gpiob.crh).downgrade(),
        gpiob.pb15.into_push_pull_output(&mut gpiob.crh).downgrade(),
        gpiob.pb5.into_push_pull_output(&mut gpiob.crl).downgrade(),
        /*
        gpiob.pb6.into_push_pull_output(&mut gpiob.crl).downgrade(),
        gpiob.pb8.into_push_pull_output(&mut gpiob.crh).downgrade(),
        gpiob.pb10.into_push_pull_output(&mut gpiob.crh).downgrade(),
        gpiob.pb11.into_push_pull_output(&mut gpiob.crh).downgrade(),
        */
    ];

    let pump = gpioa.pa4.into_push_pull_output(&mut gpioa.crl);
    //let pump_right = gpiob.pb0.into_push_pull_output(&mut gpiob.crl);

    {
        // Hardfault LED
        let mut pin = gpioc.pc15.into_push_pull_output(&mut gpioc.crh);
        pin.set_low();
        // Blinking led
        let mut _led = gpioc.pc13.into_push_pull_output(&mut gpioc.crh);
    }
    let led_communication = gpioc.pc14.into_push_pull_output(&mut gpioc.crh);

    let tirette = gpiob.pb1.into_pull_down_input(&mut gpiob.crl);

    let spi = Spi::spi1(
        chip.SPI1,
        (sclk, miso, mosi),
        &mut afio.mapr,
        Mode {
            polarity: Polarity::IdleLow,
            phase: Phase::CaptureOnFirstTransition,
        },
        1.mhz(),
        clocks,
        &mut rcc.apb2,
    );

    // Speaker
    let c1 = gpioa.pa0.into_alternate_push_pull(&mut gpioa.crl);
    let mut speaker_pwm = chip
        .TIM2
        .pwm(c1, &mut afio.mapr, 440.hz(), clocks, &mut rcc.apb1);
    speaker_pwm.enable();

    // Clignotement de la led
    let mut t_led = Timer::tim1(chip.TIM1, 5.hz(), clocks, &mut rcc.apb2);
    t_led.listen(Event::Update);
    cortex.NVIC.enable(TIM1_UP);

    //  Create a delay timer from the RCC clocks.
    let delay = Delay::new(cortex.SYST, clocks);

    (
        Robot {
            delay,
            led_communication,
            pump,
            valves: vannes,
            tirette,
            speaker: Speaker::new(speaker_pwm, clocks),
        },
        spi,
        cs,
    )
}

#[interrupt]
fn TIM1_UP() {
    static mut TOOGLE: bool = false;
    unsafe {
        (*f103::TIM1::ptr()).sr.write(|w| w.uif().clear_bit());
        if *TOOGLE {
            (*f103::GPIOC::ptr()).bsrr.write(|w| w.br13().set_bit());
        } else {
            (*f103::GPIOC::ptr()).bsrr.write(|w| w.bs13().set_bit());
        }
        *TOOGLE = !(*TOOGLE);
    }
}

#[exception]
fn HardFault(ef: &ExceptionFrame) -> ! {
    unsafe {
        (*f103::GPIOC::ptr()).bsrr.write(|w| w.bs15().set_bit());
    }
    panic!("Hard fault: {:#?}", ef);
}

//  For any unhandled interrupts, show a message on the debug console and stop.

#[exception]
fn DefaultHandler(irqn: i16) {
    unsafe {
        (*f103::GPIOC::ptr()).bsrr.write(|w| w.br15().set_bit());
    }
    panic!("Unhandled exception (IRQn = {})", irqn);
}
