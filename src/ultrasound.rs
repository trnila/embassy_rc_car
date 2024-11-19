use defmt::info;
use embassy_executor::task;
use embassy_stm32::{exti::ExtiInput, gpio::Output};
use embassy_time::{with_timeout, Instant, Timer};
use movavg::MovAvg;

use crate::ULTRASOUNDS;

#[derive(Copy, Clone)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum UltrasoundResult {
    Fail,
    Measurement(u64),
}

async fn measure_ultrasound(
    trigger: &mut Output<'static>,
    echo: &mut ExtiInput<'static>,
) -> UltrasoundResult {
    trigger.set_high();
    Timer::after_micros(10).await;
    trigger.set_low();

    let x = with_timeout(
        embassy_time::Duration::from_millis(10),
        echo.wait_for_high(),
    )
    .await;
    if x.is_err() {
        return UltrasoundResult::Fail;
    }

    let start = Instant::now();

    let x = with_timeout(embassy_time::Duration::from_millis(10), echo.wait_for_low()).await;

    if x.is_err() {
        return UltrasoundResult::Fail;
    }

    let time = Instant::now() - start;
    let res = UltrasoundResult::Measurement((time.as_micros() as f32 / 57.5 * 10.0) as u64);

    info!("{}us {}mm", time.as_micros(), res);
    res
}

#[task]
pub async fn ultrasound(mut ultrasounds: [(i32, Output<'static>, ExtiInput<'static>); 2]) {
    let mut avg: [MovAvg<u64, i64, 12>; 2] = [MovAvg::new(), MovAvg::new()];
    loop {
        let mut results = [UltrasoundResult::Fail; 6];

        for (ch, ref mut trigger, ref mut echo) in ultrasounds.iter_mut() {
            let mut result = measure_ultrasound(trigger, echo).await;

            if let UltrasoundResult::Measurement(val) = result {
                avg[*ch as usize].feed(val);
            }

            if let Ok(val) = avg[*ch as usize].try_get() {
                result = UltrasoundResult::Measurement(val);
            }

            results[*ch as usize] = result;
            info!("ultrasound {} {:?}", ch, result);
        }

        ULTRASOUNDS.signal(results);
    }
}
