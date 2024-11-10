use defmt::info;
use embassy_executor::task;
use embassy_stm32::{
    adc::Adc,
    peripherals::{ADC1, PC0},
};
use embassy_time::Timer;

use crate::KL15;

pub struct KL15 {
    adc: Adc<'static, ADC1>,
    pin: PC0,
}

impl KL15 {
    pub fn new(adc: Adc<'static, ADC1>, pin: PC0) -> Self {
        Self { adc, pin }
    }

    pub fn read(&mut self) -> u16 {
        // read vRef
        let mut vrefint = self.adc.enable_vrefint();
        let vrefint_sample = self.adc.blocking_read(&mut vrefint);

        // read raw ADC
        let sample = self.adc.blocking_read(&mut self.pin);

        // convert ADC to mV
        const VREFINT_MV: u32 = 1212; // mV
        let millivolts = u32::from(sample) * VREFINT_MV / u32::from(vrefint_sample);

        // calculate voltage before divider
        let r1: u32 = 4700;
        let r2: u32 = 1500;
        (millivolts * (r1 + r2) / r2) as u16
    }
}

#[task]
pub async fn measure_kl15(mut kl15: KL15) {
    loop {
        KL15.signal(kl15.read());
        Timer::after_millis(100).await;
    }
}
