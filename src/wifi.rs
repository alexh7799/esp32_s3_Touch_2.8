use crate::config;
use anyhow::Context;
use std::time::Duration;
use esp_idf_svc::eventloop::EspSystemEventLoop;
use esp_idf_svc::nvs::EspDefaultNvsPartition;
use esp_idf_svc::wifi::{BlockingWifi, ClientConfiguration, Configuration, EspWifi};

pub fn make_wifi(
    modem: esp_idf_hal::modem::Modem,
    sysloop: EspSystemEventLoop,
    nvs: EspDefaultNvsPartition,
) -> anyhow::Result<BlockingWifi<EspWifi<'static>>> {
    let wifi = EspWifi::new(modem, sysloop.clone(), Some(nvs)).context("EspWifi::new failed")?;
    BlockingWifi::wrap(wifi, sysloop).context("BlockingWifi::wrap failed")
}

pub fn connect_wifi(wifi: &mut BlockingWifi<EspWifi<'static>>) -> anyhow::Result<()> {
    wifi.set_configuration(&Configuration::Client(ClientConfiguration {
        ssid: config::WIFI_SSID
            .try_into()
            .map_err(|_| anyhow::anyhow!("WIFI_SSID too long"))?,
        password: config::WIFI_PASS
            .try_into()
            .map_err(|_| anyhow::anyhow!("WIFI_PASS too long"))?,
        ..Default::default()
    })).context("wifi set_configuration failed")?;
    wifi.start().context("wifi start failed")?;
    loop {
        match wifi.connect() {
            Ok(()) => break,
            Err(error) => {
                let _ = wifi.disconnect();
                std::thread::sleep(Duration::from_secs(5));
            }
        }
    }
    wifi.wait_netif_up().context("wifi wait_netif_up failed")?;
    Ok(())
}
