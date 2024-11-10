#![allow(unused_imports)]
#![allow(non_snake_case)]
#![allow(dead_code)]
#![allow(unused_variables)]
#![allow(non_upper_case_globals)]
#![no_std]
#![no_main]

use core::time::Duration;

use cortex_m::singleton;
use defmt::*;
use embassy_executor::Spawner;
use embassy_stm32::adc::Adc;
use embassy_stm32::adc::SampleTime;
use embassy_stm32::exti::ExtiInput;
use embassy_stm32::gpio::Level;
use embassy_stm32::gpio::Output;
use embassy_stm32::gpio::OutputType;
use embassy_stm32::gpio::Pull;
use embassy_stm32::gpio::Speed;
use embassy_stm32::peripherals::*;
use embassy_stm32::time::hz;
use embassy_stm32::timer::qei::Qei;
use embassy_stm32::timer::qei::QeiPin;
use embassy_stm32::timer::simple_pwm::PwmPin;
use embassy_stm32::timer::simple_pwm::SimplePwm;
use embassy_stm32::timer::Channel;
use embassy_stm32::usart::BufferedUart;
use embassy_stm32::usart::Uart;
use embassy_stm32::{bind_interrupts, can, usart, Config};
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::signal::Signal;
use {defmt_rtt as _, panic_probe as _};

mod blinky;
mod can_scheduler;
mod kl15;
mod lin_master;
mod messages;
mod rotary_encoder;
mod servo;
mod ultrasound;

const SLAVE: bool = false;

bind_interrupts!(struct Irqs {
    FDCAN1_IT0 => can::IT0InterruptHandler<FDCAN1>;
    FDCAN1_IT1 => can::IT1InterruptHandler<FDCAN1>;
});

bind_interrupts!(struct UARTIRqs {
    UART4 => usart::BufferedInterruptHandler<UART4>;
});

static SPEED: Signal<CriticalSectionRawMutex, f32> = Signal::new();
static ULTRASOUNDS: Signal<CriticalSectionRawMutex, [ultrasound::UltrasoundResult; 6]> =
    Signal::new();
static SERVO_DEGREE: Signal<CriticalSectionRawMutex, f32> = Signal::new();
static KL15: Signal<CriticalSectionRawMutex, u16> = Signal::new();

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    let mut config = Config::default();
    {
        use embassy_stm32::rcc::*;
        config.rcc.pll = Some(Pll {
            source: PllSource::HSI,
            prediv: PllPreDiv::DIV4,
            mul: PllMul::MUL85,
            divp: Some(PllPDiv::DIV20),
            divq: Some(PllQDiv::DIV2),
            divr: Some(PllRDiv::DIV2),
        });
        config.rcc.mux.adc12sel = mux::Adcsel::PLL1_P;
        config.rcc.mux.fdcansel = mux::Fdcansel::PCLK1;
        config.rcc.sys = Sysclk::PLL1_R;
    }
    let peripherals = embassy_stm32::init(config);

    let qei = Qei::new(
        peripherals.TIM2,
        QeiPin::new_ch1(peripherals.PA0),
        QeiPin::new_ch2(peripherals.PA1),
    );

    let mut can =
        can::CanConfigurator::new(peripherals.FDCAN1, peripherals.PB8, peripherals.PA12, Irqs);

    can.properties().set_extended_filter(
        can::filter::ExtendedFilterSlot::_0,
        can::filter::ExtendedFilter::accept_all_into_fifo1(),
    );

    can.set_bitrate(500_000);
    can.set_fd_data_bitrate(1_000_000, false);
    let can = can.start(can::OperatingMode::NormalOperationMode);

    let config = {
        let mut config = usart::Config::default();
        config.baudrate = 19200;
        config
    };

    let (tx, rx, _) = can.split();

    let pwm_time = Duration::from_millis(20);
    let pwm_freq = hz((Duration::from_secs(1).as_micros() / pwm_time.as_micros()) as u32);

    let ch1 = PwmPin::new_ch1(peripherals.PA6, OutputType::PushPull);
    let pwm = SimplePwm::new(
        peripherals.TIM3,
        Some(ch1),
        None,
        None,
        None,
        pwm_freq,
        Default::default(),
    );
    let min_us = Duration::from_micros(1075);
    let max_us = Duration::from_micros(1896);
    let servo = servo::Servo::new(pwm, Channel::Ch1, pwm_time, min_us, max_us);

    let mut adc = Adc::new(peripherals.ADC1);
    adc.set_sample_time(SampleTime::CYCLES640_5);
    let kl15 = kl15::KL15::new(adc, peripherals.PC0);

    let led_pin = Output::new(peripherals.PA11, Level::Low, Speed::Low);

    let ultrasounds = [
        (
            0,
            Output::new(peripherals.PB9, Level::Low, Speed::VeryHigh),
            ExtiInput::new(peripherals.PB0, peripherals.EXTI0, Pull::Down),
        ),
        (
            1,
            Output::new(peripherals.PB10, Level::Low, Speed::VeryHigh),
            ExtiInput::new(peripherals.PB1, peripherals.EXTI1, Pull::Down),
        ),
        (
            2,
            Output::new(peripherals.PB11, Level::Low, Speed::VeryHigh),
            ExtiInput::new(peripherals.PB2, peripherals.EXTI2, Pull::Down),
        ),
        (
            3,
            Output::new(peripherals.PB12, Level::Low, Speed::VeryHigh),
            ExtiInput::new(peripherals.PB3, peripherals.EXTI3, Pull::Down),
        ),
        (
            4,
            Output::new(peripherals.PB13, Level::Low, Speed::VeryHigh),
            ExtiInput::new(peripherals.PB4, peripherals.EXTI4, Pull::Down),
        ),
        (
            5,
            Output::new(peripherals.PB14, Level::Low, Speed::VeryHigh),
            ExtiInput::new(peripherals.PB5, peripherals.EXTI5, Pull::Down),
        ),
    ];

    spawner.spawn(can_scheduler::can_rx(rx)).unwrap();
    spawner.spawn(can_scheduler::can_tx(tx)).unwrap();
    //spawner.spawn(servo_tester(servo)).unwrap();
    spawner.spawn(servo::servo_task(servo)).unwrap();
    spawner.spawn(kl15::measure_kl15(kl15)).unwrap();
    spawner.spawn(blinky::blinky(led_pin)).unwrap();
    spawner.spawn(ultrasound::ultrasound(ultrasounds)).unwrap();
    spawner
        .spawn(rotary_encoder::rotary_encoder_task(qei))
        .unwrap();

    let tx_buf: &mut [u8; 32] = singleton!(TX_BUF: [u8; 32] = [0; 32]).unwrap();
    let rx_buf: &mut [u8; 32] = singleton!(RX_BUF: [u8; 32] = [0; 32]).unwrap();
    let uart = BufferedUart::new(
        peripherals.UART4,
        UARTIRqs,
        peripherals.PC11,
        peripherals.PC10,
        tx_buf,
        rx_buf,
        config,
    )
    .unwrap();

    let lin = lin_master::LinMaster { driver: uart };
    spawner.spawn(lin_master::lin_scheduler(lin)).unwrap();
}
