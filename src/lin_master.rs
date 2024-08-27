use defmt::{info, println};
use embassy_executor::task;
use embassy_stm32::{
    mode::Async,
    usart::{BufferedUart, Uart},
};
use embassy_time::{with_timeout, Duration, Timer};
use embedded_io_async::{Read, Write};
use lin_bus::{Frame, PID};

pub struct LinMaster {
    pub driver: BufferedUart<'static>,
}

impl LinMaster {
    async fn write_frame(&mut self, frame: &Frame) -> Result<(), lin_bus::Error> {
        self.send_header(frame.get_pid()).await?;
        self.write(frame.get_data_with_checksum()).await
    }

    async fn read_frame(&mut self, pid: PID, data_length: usize) -> Result<Frame, lin_bus::Error> {
        assert!(data_length <= 8, "Maximum data length is 8 bytes");
        self.send_header(pid).await?;

        let mut data = [0; 9];
        self.read(&mut data[0..=data_length]).await?;

        let frame = Frame::from_data(pid, &data[0..data_length]);

        if frame.get_checksum() == data[data_length] {
            Ok(frame)
        } else {
            Err(lin_bus::Error::Checksum)
        }
    }

    async fn send_header(&mut self, pid: lin_bus::PID) -> Result<(), lin_bus::Error> {
        let timeout: Duration = Duration::from_secs(1);

        self.driver.send_break();
        let mut inbuffer = [0u8; 1];
        match with_timeout(timeout, self.driver.read(&mut inbuffer)).await {
            Ok(Ok(n)) => {
                //info!("break got: {} {}", n, inbuffer);
            }
            Ok(Err(e)) => {
                info!("break resp: {}", e);
            }
            Err(_) => return Err(lin_bus::Error::Timeout),
        };
        //crate::assert!(Err(usart::Error::Framing) == err);

        let buffer = [0x55, pid.get()];
        with_timeout(timeout, self.driver.write_all(&buffer))
            .await
            .unwrap()
            .unwrap();

        let mut inbuffer = [0u8; 2];
        with_timeout(timeout, self.driver.read_exact(&mut inbuffer))
            .await
            .unwrap()
            .unwrap();
        crate::assert!(buffer == inbuffer);

        Ok(())
    }

    async fn read(&mut self, buf: &mut [u8]) -> Result<(), lin_bus::Error> {
        let res = with_timeout(
            embassy_time::Duration::from_millis(50),
            self.driver.read_exact(buf),
        )
        .await;

        match res {
            Ok(Ok(_)) => Ok(()),
            Ok(Err(_)) => Err(lin_bus::Error::PhysicalBus),
            Err(_) => Err(lin_bus::Error::Timeout),
        }
    }

    async fn write(&mut self, data: &[u8]) -> Result<(), lin_bus::Error> {
        let timeout: Duration = Duration::from_secs(1);

        with_timeout(timeout, self.driver.write_all(data))
            .await
            .unwrap()
            .unwrap();

        for _ in 0..data.len() {
            let mut buffer = [0u8; 1];
            let _ = with_timeout(timeout, self.driver.read(&mut buffer)).await;
        }
        Ok(())
    }
}

#[task]
pub async fn lin_scheduler(mut lin: LinMaster) {
    loop {
        let f = lin_bus::Frame::from_data(PID::from_id(0x22), &[0x11, 0x22, 0x33, 0x44]);
        lin.write_frame(&f).await.unwrap();

        Timer::after_millis(100).await;

        let fr = lin.read_frame(PID::from_id(0x08), 4).await;
        match fr {
            Ok(fr) => info!("LIN RX {} {:?}", fr.get_pid().get_id(), fr.get_data()),
            Err(err) => info!(
                "Error reading LIN: {}",
                match err {
                    lin_bus::Error::Timeout => "timeout",
                    lin_bus::Error::PhysicalBus => "physicalbus",
                    lin_bus::Error::Checksum => "checksum",
                }
            ),
        };

        Timer::after_millis(100).await;
    }
}
