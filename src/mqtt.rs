use crate::config;
use crate::handle_inbound;
use crate::{UiInbound, UiOutbound};
use anyhow::Context;
use esp_idf_svc::mqtt::client::{
    EspMqttClient, EspMqttConnection, EventPayload, MqttClientConfiguration, QoS,
};
use log::{error, info, warn};
use std::sync::mpsc::Sender;
use std::time::Duration;

pub fn mqtt_worker(conn: &mut EspMqttConnection, tx: Sender<UiInbound>) {
    loop {
        match conn.next() {
            Ok(ev) => match ev.payload() {
                EventPayload::Connected(_) => {
                    info!("mqtt connected");
                    let _ = tx.send(UiInbound::Online(true));
                }
                EventPayload::Disconnected => {
                    warn!("mqtt disconnected");
                    let _ = tx.send(UiInbound::Online(false));
                }
                EventPayload::Received { topic, data, .. } => {
                    if let Some(msg) = handle_inbound(topic.as_deref(), data) {
                        let _ = tx.send(msg);
                    }
                }
                EventPayload::Error(e) => error!("mqtt error: {e:?}"),
                _ => {}
            },
            Err(e) => warn!("mqtt conn.next error: {e:?}"),
        }
    }
}

pub fn make_mqtt() -> anyhow::Result<(EspMqttClient<'static>, EspMqttConnection)> {
    let cfg = MqttClientConfiguration {
        client_id: Some(config::MQTT_CLIENT_ID),
        username: Some(config::MQTT_USER),
        password: Some(config::MQTT_PASS),
        keep_alive_interval: Some(Duration::from_secs(30)),
        lwt: Some(esp_idf_svc::mqtt::client::LwtConfiguration {
            topic: config::TOPIC_STATE,
            payload: b"offline",
            qos: QoS::AtLeastOnce,
            retain: true,
        }),
        ..Default::default()
    };
    EspMqttClient::new(config::MQTT_BROKER, &cfg).context("EspMqttClient::new failed")
}

pub fn publish_outbound(client: &mut EspMqttClient<'_>, cmd: UiOutbound) -> anyhow::Result<()> {
    match cmd {
        UiOutbound::SetIsOn(v) => {
            let payload: &[u8] = if v { b"true" } else { b"false" };
            client.enqueue(config::TOPIC_ISON_STATE, QoS::AtMostOnce, true, payload)?;
        }
        UiOutbound::SetBrightness(v) => {
            let v = v.clamp(1, 100);
            let payload = format!("{v}");
            client.enqueue(
                config::TOPIC_BRIGHTNESS_STATE,
                QoS::AtMostOnce,
                true,
                payload.as_bytes(),
            )?;
        }
    }
    Ok(())
}
