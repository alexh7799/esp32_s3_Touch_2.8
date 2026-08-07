use crate::test_config;

pub const WIFI_SSID: &str = test_config::WIFI_SSID;
pub const WIFI_PASS: &str = test_config::WIFI_PASS;
pub const NETWORK_SEED: u64 = 0x1234_5678_9ABC_DEF0;

pub const MQTT_BROKER: &str = test_config::MQTT_BROKER;

pub const MQTT_USER: &str = test_config::MQTT_USER;
pub const MQTT_PASS: &str = test_config::MQTT_PASS;

pub const MQTT_CLIENT_ID: &str = test_config::MQTT_CLIENT_ID;

pub const TOPIC_BRIGHTNESS: &str = test_config::TOPIC_BRIGHTNESS;
pub const TOPIC_ISON: &str = test_config::TOPIC_ISON;
pub const TOPIC_BRIGHTNESS_STATE: &str = test_config::TOPIC_BRIGHTNESS_STATE;
pub const TOPIC_ISON_STATE: &str = test_config::TOPIC_ISON_STATE;
pub const TOPIC_STATE: &str = test_config::TOPIC_STATE;
