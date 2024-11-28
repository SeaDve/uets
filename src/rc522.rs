use anyhow::{anyhow, Result};
use embedded_hal::delay::DelayNs;
use embedded_hal_bus::spi::ExclusiveDevice;
use gtk::{glib, prelude::*, subclass::prelude::*};
use linux_embedded_hal::{
    spidev::{SpiModeFlags, SpidevOptions},
    Delay, SpidevBus, SysfsPin,
};
use mfrc522::{
    comm::{blocking::spi::SpiInterface, Interface},
    Initialized, Mfrc522,
};

mod imp {
    use super::*;

    #[derive(Default)]
    pub struct Rc522 {}

    #[glib::object_subclass]
    impl ObjectSubclass for Rc522 {
        const NAME: &'static str = "UetsRc522";
        type Type = super::Rc522;
    }

    impl ObjectImpl for Rc522 {
        fn constructed(&self) {
            self.parent_constructed();

            let obj = self.obj();

            if let Err(err) = obj.setup_device() {
                tracing::debug!("Failed to setup device: {:?}", err);
            }
        }
    }
}

glib::wrapper! {
    pub struct Rc522(ObjectSubclass<imp::Rc522>);
}

impl Rc522 {
    pub fn new() -> Self {
        glib::Object::new()
    }

    fn setup_device(&self) -> Result<()> {
        let mut delay = Delay;

        let mut spi = SpidevBus::open("/dev/spidev0.0")?;
        let options = SpidevOptions::new()
            .max_speed_hz(1_000_000)
            .mode(SpiModeFlags::SPI_MODE_0 | SpiModeFlags::SPI_NO_CS)
            .build();
        spi.configure(&options)?;

        let pin = SysfsPin::new(22);
        pin.export()?;
        while !pin.is_exported() {}
        delay.delay_ms(500u32); // delay sometimes necessary because `is_exported()` returns too early?
        let pin = pin.into_output_pin(embedded_hal::digital::PinState::High)?;

        let spi = ExclusiveDevice::new(spi, pin, Delay)?;
        let itf = SpiInterface::new(spi);
        let mut mfrc522 = Mfrc522::new(itf)
            .init()
            .map_err(|err| anyhow!("{:?}", err))?;

        let vers = mfrc522.version().map_err(|err| anyhow!("{:?}", err))?;

        println!("VERSION: 0x{:x}", vers);

        assert!(vers == 0x91 || vers == 0x92);

        loop {
            const CARD_UID: [u8; 4] = [34, 246, 178, 171];
            const TAG_UID: [u8; 4] = [128, 170, 179, 76];

            if let Ok(atqa) = mfrc522.reqa() {
                if let Ok(uid) = mfrc522.select(&atqa) {
                    println!("UID: {:?}", uid.as_bytes());

                    if uid.as_bytes() == CARD_UID {
                        println!("CARD");
                    } else if uid.as_bytes() == TAG_UID {
                        println!("TAG");
                    }

                    handle_authenticate(&mut mfrc522, &uid, |m| {
                        let data = m.mf_read(1).map_err(|err| anyhow!("{:?}", err))?;
                        println!("read {:?}", data);
                        Ok(())
                    })
                    .ok();
                }
            }

            delay.delay_ms(1000u32);
        }

        Ok(())
    }
}

impl Default for Rc522 {
    fn default() -> Self {
        Self::new()
    }
}

fn handle_authenticate<E, COMM: Interface<Error = E>, F>(
    mfrc522: &mut Mfrc522<COMM, Initialized>,
    uid: &mfrc522::Uid,
    action: F,
) -> Result<()>
where
    F: FnOnce(&mut Mfrc522<COMM, Initialized>) -> Result<()>,
    E: std::fmt::Debug + std::marker::Sync + std::marker::Send + 'static,
{
    // Use *default* key, this should work on new/empty cards
    let key = [0xFF; 6];
    if mfrc522.mf_authenticate(uid, 1, &key).is_ok() {
        action(mfrc522)?;
    } else {
        println!("Could not authenticate");
    }

    mfrc522.hlta().map_err(|err| anyhow!("{:?}", err))?;
    mfrc522.stop_crypto1().map_err(|err| anyhow!("{:?}", err))?;
    Ok(())
}
