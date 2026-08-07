pub mod config;
pub mod cst328;
pub mod dht11;
pub mod mqtt;
pub mod slint_backend;
pub mod st7789;
pub mod test_config;
pub mod wifi;

#[derive(Debug, Clone, Copy)]
pub enum UiInbound {
    IsOn(bool),
    Brightness(u8),
    Online(bool),
    Temperature(u8),
    Humidity(u8),
}

#[derive(Debug, Clone, Copy)]
pub enum UiOutbound {
    SetIsOn(bool),
    SetBrightness(u8),
}

pub fn handle_inbound(topic: Option<&str>, data: &[u8]) -> Option<UiInbound> {
    let topic = topic?;

    if topic == crate::config::TOPIC_ISON {
        let s = core::str::from_utf8(data).ok()?.trim();
        let v = s.eq_ignore_ascii_case("true") || s == "1";
        return Some(UiInbound::IsOn(v));
    }

    if topic == crate::config::TOPIC_BRIGHTNESS {
        let text = core::str::from_utf8(data).ok()?.trim();
        let value = text.parse::<u8>().ok()?;
        if (1..=100).contains(&value) {
            return Some(UiInbound::Brightness(value));
        }
    }

    None
}
