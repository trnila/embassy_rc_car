use core::time::Duration;

use cortex_m::prelude::_embedded_hal_Pwm;
use defmt::info;
use embassy_executor::task;
use embassy_stm32::{
    peripherals::TIM3,
    timer::{simple_pwm::SimplePwm, Channel, GeneralInstance4Channel},
};
use embassy_time::Timer;

use crate::SERVO_DEGREE;

pub struct Servo<T: GeneralInstance4Channel> {
    pwm: SimplePwm<'static, T>,
    channel: Channel,
    period: Duration,
    min: Duration,
    max: Duration,
}

impl<T: GeneralInstance4Channel> Servo<T> {
    pub fn new(
        pwm: SimplePwm<'static, T>,
        channel: Channel,
        period: Duration,
        min: Duration,
        max: Duration,
    ) -> Self {
        let mut servo = Self {
            pwm,
            channel,
            period,
            min,
            max,
        };
        servo.set(0);
        servo.enable();
        servo
    }

    pub fn enable(&mut self) {
        self.pwm.enable(self.channel);
    }

    pub fn set(&mut self, percent: i8) {
        let percent = percent.clamp(-100, 100);

        let tick_us = self.period / self.pwm.get_max_duty();
        let half = (self.max - self.min) / 2;
        let center = self.min + half;

        let shift = percent as f32 * half.as_secs_f32() / 100.0;
        let calculated_time = Duration::from_secs_f32(center.as_secs_f32() + shift);

        let duty = (calculated_time.as_secs_f32() / tick_us.as_secs_f32()) as u32;
        info!("{}", duty);

        self.pwm.set_duty(self.channel, duty);
    }
}

#[task]
pub async fn servo_task(mut servo: Servo<TIM3>) {
    loop {
        let degree = SERVO_DEGREE.wait().await;
        info!("Servo req to {}", degree);
        servo.set(degree as i8);
    }
}

#[task]
pub async fn servo_tester(mut servo: Servo<TIM3>) {
    loop {
        for i in -100..100 {
            servo.set(i);
            Timer::after_millis(300).await;
        }
    }
}
