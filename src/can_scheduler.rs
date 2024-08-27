use defmt::{error, info};
use embassy_executor::task;
use embassy_stm32::can::{
    frame::{FdFrame, Header},
    CanRx, CanTx,
};
use embassy_time::Timer;

use crate::{
    messages::{self, Messages},
    ultrasound::UltrasoundResult,
    SERVO_DEGREE, SPEED, ULTRASOUNDS,
};

fn to_embassy_frame<F: embedded_can::Frame>(frame: F) -> FdFrame {
    let hdr = Header::new(frame.id(), frame.dlc() as u8, false);
    FdFrame::new(hdr, frame.data()).unwrap()
}

#[task]
pub async fn can_rx(mut can_rx: CanRx<'static>) {
    let mut last_read_ts = embassy_time::Instant::now();

    loop {
        match can_rx.read().await {
            Ok(envelope) => {
                let (ts, rx_frame) = (envelope.ts, envelope.frame);
                let delta = (ts - last_read_ts).as_millis();
                last_read_ts = ts;

                let payload = &rx_frame.data()[..rx_frame.header().len() as usize];
                info!(
                    "Rx: {} {:02x} --- {}ms",
                    rx_frame.header().len(),
                    payload,
                    delta,
                );

                let msg = messages::Messages::from_can_message(*rx_frame.id(), payload);
                match msg {
                    Err(err) => info!("CAN RX err"),
                    Ok(frame) => match frame {
                        Messages::WheelAngle(frame) => {
                            SERVO_DEGREE.signal(frame.wheel_angle());
                            info!("RX wheel angle: {}", frame.wheel_angle());
                        }
                        _ => info!("RX unneeded message"),
                    },
                };
            }
            Err(_err) => error!("Error in frame, {:?}", _err),
        }
    }
}

#[task]
pub async fn can_tx(mut can_tx: CanTx<'static>) {
    let mut i = 0u8;

    let mut msg_rear = messages::RearDist::new(3, 2, 1).unwrap();
    let mut msg_front = messages::FrontDist::new(3, 2, 1).unwrap();
    let mut msg_speed = messages::SpeedKmh::new(i as f32).unwrap();

    loop {
        if let Some(val) = SPEED.try_take() {
            msg_speed.set_speed_kmh(val).unwrap();
        }

        if let Some(results) = ULTRASOUNDS.try_take() {
            let map = |val| match val {
                UltrasoundResult::Fail => 0x0u16,
                UltrasoundResult::Measurement(val) => val as u16,
            };
            msg_front.set_front_dist_1(map(results[0])).unwrap();
            msg_front.set_front_dist_2(map(results[1])).unwrap();
            msg_front.set_front_dist_3(map(results[2])).unwrap();
            msg_rear.set_rear_dist_1(map(results[3])).unwrap();
            msg_rear.set_rear_dist_2(map(results[4])).unwrap();
            msg_rear.set_rear_dist_3(map(results[5])).unwrap();
        }

        can_tx.write_fd(&to_embassy_frame(msg_rear)).await;
        can_tx.write_fd(&to_embassy_frame(msg_front)).await;
        can_tx.write_fd(&to_embassy_frame(msg_speed)).await;

        Timer::after_millis(250).await;
        i = i.wrapping_add(1);
    }
}
