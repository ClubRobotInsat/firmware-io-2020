#![no_main]
#![no_std]
#![allow(unused_imports)]

use crate::f103::Peripherals;
use crate::hal::stm32 as f103;
use cortex_m::Peripherals as CortexPeripherals;
use cortex_m_rt::entry;
use embedded_hal::digital::{InputPin, OutputPin, StatefulOutputPin, ToggleableOutputPin};
use librobot::eth::get_main_computer_ip;
#[allow(unused_imports)]
use panic_semihosting;
use stm32f1xx_hal as hal;
use w5500::*;
mod robot;
use crate::hal::device::SPI1;
use crate::hal::spi::Spi;
use crate::robot::init_peripherals;
use crate::robot::Robot;
use crate::robot::SpiPins;
use core::cmp::min;
use cortex_m::asm;
use cortex_m_semihosting::hprintln;
use embedded_hal::blocking::delay::DelayMs;
use embedded_hal::spi::FullDuplex;
use heapless::consts::U2048;
use heapless::{String, Vec};
use librobot::transmission::id::*;
use librobot::transmission::io::IOState;
use librobot::transmission::{
    eth::{init_eth, listen_on, SOCKET_UDP},
    io::{BuzzerState, Pneumatic, TriggerState, IO},
    Jsonizable,
};
use pwm_speaker::songs::*;
use w5500::Socket::*;

fn send_tirette_state<T, K>(
    robot: &mut Robot,
    spi: &mut Spi<T, K>,
    eth: &mut W5500,
    buzzer_state: &BuzzerState,
    ip: &IpAddress,
) where
    Spi<T, K>: FullDuplex<u8>,
{
    let tirette = if robot.tirette.is_low() {
        TriggerState::Waiting
    } else {
        TriggerState::Triggered
    };

    let state = IO {
        buzzer: *buzzer_state,
        tirette,
    };

    if let Ok(data) = state.to_string::<U2048>() {
        robot.led_communication.set_low();
        if let Ok(_) = eth.send_udp(
            spi,
            Socket0,
            ELEC_LISTENING_PORT + ID_IO,
            ip,
            INFO_LISTENING_PORT + ID_IO,
            &data.as_bytes(),
        ) {}
    }
}

fn send_pneumatic_state<T, K>(
    robot: &mut Robot,
    spi: &mut Spi<T, K>,
    eth: &mut W5500,
    ip: &IpAddress,
) where
    Spi<T, K>: FullDuplex<u8>,
{
    let pump = if robot.pump.is_set_high() {
        IOState::On
    } else {
        IOState::Off
    };

    let mut valves = [IOState::On; 4];
    for (state, valve) in robot.valves.iter().zip(valves.iter_mut()) {
        *valve = if state.is_set_high() {
            IOState::On
        } else {
            IOState::Off
        };
    }

    let state = Pneumatic { pump, valves };

    if let Ok(data) = state.to_string::<U2048>() {
        if let Ok(_) = eth.send_udp(
            spi,
            Socket1,
            ELEC_LISTENING_PORT + ID_PNEUMATIC,
            ip,
            INFO_LISTENING_PORT + ID_PNEUMATIC,
            &data.as_bytes(),
        ) {}
    }
}

fn toogle<T>(state: &mut bool, pin: &mut T)
where
    T: OutputPin,
{
    if *state {
        pin.set_high();
    } else {
        pin.set_low();
    }
    *state = !(*state);
}

#[entry]
fn main() -> ! {
    let chip = Peripherals::take().unwrap();
    let cortex = CortexPeripherals::take().unwrap();
    let (mut robot, mut spi, mut cs): (Robot, Spi<SPI1, SpiPins>, _) =
        init_peripherals(chip, cortex);
    let mut eth = { W5500::new(&mut spi, &mut cs) };
    {
        init_eth(
            &mut eth,
            &mut spi,
            min(ID_PNEUMATIC as u8, ID_IO as u8),
            min(ID_PNEUMATIC as u8, ID_IO as u8),
        );
        // IO
        listen_on(&mut eth, &mut spi, ID_IO + ELEC_LISTENING_PORT, Socket0);
        listen_on(
            &mut eth,
            &mut spi,
            ID_PNEUMATIC + ELEC_LISTENING_PORT,
            Socket1,
        );
    }
    let mut buffer = [0u8; 2048];

    let mut buzzer_state = BuzzerState::Rest;

    let mut tirette_already_detected = false;

    let mut led_state = false;

    robot.led_communication.set_low();

    robot.speaker.play_score(&SUCCESS_SONG, &mut robot.delay);

    loop {
        if robot.tirette.is_low() && !tirette_already_detected {
            tirette_already_detected = true;
            send_tirette_state(
                &mut robot,
                &mut spi,
                &mut eth,
                &buzzer_state,
                &get_main_computer_ip(),
            )
        } else if robot.tirette.is_high() && tirette_already_detected {
            tirette_already_detected = false;
            send_tirette_state(
                &mut robot,
                &mut spi,
                &mut eth,
                &buzzer_state,
                &get_main_computer_ip(),
            )
        }

        if let Ok(Some((ip, _, size))) = eth.try_receive_udp(&mut spi, Socket0, &mut buffer) {
            use BuzzerState::*;
            /*S
            hprintln!(
                "IO data: {:#x?}",
                core::str::from_utf8(&buffer[0..(size - 1)]).unwrap()
            )
            .unwrap();
            */
            match IO::from_json_slice(&buffer[0..size]) {
                Ok(io) => {
                    toogle(&mut led_state, &mut robot.led_communication);
                    match (io.buzzer, buzzer_state) {
                        (PlayErrorSound, Rest) => {
                            robot.speaker.play_score(&FAILURE_SONG, &mut robot.delay);
                            buzzer_state = PlayErrorSound;
                        }
                        (PlaySuccessSound, Rest) => {
                            robot.speaker.play_score(&SUCCESS_SONG, &mut robot.delay);
                            buzzer_state = PlaySuccessSound;
                        }

                        (Rest, _) => {
                            buzzer_state = Rest;
                        }

                        _ => {}
                    }
                    send_tirette_state(&mut robot, &mut spi, &mut eth, &mut buzzer_state, &ip);
                }
                Err(_) => {
                    //panic!("{:#?}", e)
                }
            }
        }
        if let Ok(Some((ip, _, size))) = eth.try_receive_udp(&mut spi, Socket1, &mut buffer) {
            /*hprintln!(
                "PNEUM data: {:#x?}",
                core::str::from_utf8(&buffer[0..size]).unwrap()
            )
            .unwrap();*/
            //hprintln!("1.0\n").unwrap();
            match Pneumatic::from_json_slice(&buffer[0..size]) {
                Ok(pneumatic) => {
                    //hprintln!("1.1\n").unwrap();

                    // Gestion des vannes
                    toogle(&mut led_state, &mut robot.led_communication);

                    for (state, valve) in pneumatic.valves.iter().zip(robot.valves.iter_mut()) {
                        if let IOState::On = state {
                            valve.set_high();
                        } else {
                            valve.set_low();
                        }
                    }

                    // Gestion des pompes
                    if let IOState::On = pneumatic.pump {
                        robot.pump.set_high();
                    } else {
                        robot.pump.set_low();
                    }

                    send_pneumatic_state(&mut robot, &mut spi, &mut eth, &ip);
                }
                Err(_e) => {
                    //hprintln!("1.2\n").unwrap();
                    //panic!("{}, {:#?}", String::from_utf8(str_vec).unwrap(), e)
                }
            }
        }
    }
}
