use defmt::info;
use embassy_executor::task;
use embassy_stm32::{peripherals::TIM2, timer::qei::Qei};
use embassy_time::Timer;

use crate::SPEED;

#[task]
pub async fn rotary_encoder_task(qei: Qei<'static, TIM2>) {
    const TICKS_PER_CM: f32 = 61.5;
    const PERIOD_MS: u64 = 50;

    let mut prev_counter = 0;

    loop {
        let now = qei.count();

        let elapsed_ticks = now.abs_diff(prev_counter);

        let direction = if now > prev_counter { 1.0 } else { -1.0 };
        let v_cm_per_hour =
            elapsed_ticks as f32 / (TICKS_PER_CM * PERIOD_MS as f32 / 1000.0) * 3600.0 * direction;
        let km_per_hour = v_cm_per_hour / 100_000.0;

        info!("{}", v_cm_per_hour);

        SPEED.signal(km_per_hour);
        prev_counter = now;
        Timer::after_millis(PERIOD_MS).await;
    }
}
