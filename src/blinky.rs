use embassy_executor::task;
use embassy_stm32::gpio::Output;
use embassy_time::Timer;

#[task]
pub async fn blinky(mut led_pin: Output<'static>) {
    loop {
        led_pin.set_high();
        Timer::after_millis(250).await;
        led_pin.set_low();
        Timer::after_millis(250).await;
    }
}
