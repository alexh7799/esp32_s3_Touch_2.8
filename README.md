# ESP 32 Touch LCD 2.8 Smart Home Room Controller

A Rust-based touch controller for switching and dimming a smart home light.
The device is designed for a 2.8-inch touch display and communicates with the smart home system through MQTT.

## Overview

This project turns an ESP32-S3 into a dedicated wall or desktop controller for a dimmable light. The user interface is rendered directly on an ST7789 display and operated through a CST328 capacitive touch controller.

The device connects to a Wi-Fi network, communicates with an MQTT broker, and provides controls for:

- Turning the light on and off
- Adjusting the brightness from 1 to 100 percent
- Displaying the current connection status
- Receiving state updates from the smart home system

If the Wi-Fi connection is unavailable, the device remains running and continues trying to reconnect instead of terminating the application.

## Features

- Rust application running on an ESP32-S3
- 2.8-inch ST7789 color display
- CST328 capacitive touch input
- Slint-based user interface
- Wi-Fi connectivity through ESP-IDF
- MQTT communication for light control and state updates
- Brightness control from 1 to 100 percent
- Automatic Wi-Fi connection retry
- Custom software-rendered display backend
- Release builds for the `xtensa-esp32s3-espidf` target

## Hardware

The current hardware configuration uses the following connections:

| Function | GPIO |
|---|---:|
| Display DC | GPIO41 |
| Display Reset | GPIO39 |
| Display Backlight | GPIO5 |
| Display SPI SCK | GPIO40 |
| Display SPI MOSI | GPIO45 |
| Display SPI MISO | GPIO46 |
| Display SPI CS | GPIO42 |
| Touch SDA | GPIO1 |
| Touch SCL | GPIO3 |

The target device is an ESP32-S3 with 16 MB of flash memory.

## Software Stack

- Rust
- ESP-IDF 5.3.2
- `esp-idf-hal`
- `esp-idf-svc`
- Slint
- MQTT
- ST7789 display driver
- CST328 touch controller driver

## MQTT Communication

The MQTT topics are defined in the project configuration and can be adjusted in `src/config.rs`.

The controller uses separate topics for commands, state updates, and device status:

| Purpose | Topic constant |
|---|---|
| Light on/off command | `TOPIC_ISON` |
| Brightness command | `TOPIC_BRIGHTNESS` |
| Device status | `TOPIC_STATE` |
| Light on/off state | `TOPIC_ISON_STATE` |
| Brightness state | `TOPIC_BRIGHTNESS_STATE` |

### Incoming messages

The device accepts the following payload formats:

- On/off: `true`, `false`, `1`, or `0`
- Brightness: an integer from `1` to `100`

Incoming MQTT messages update the user interface.

### Outgoing messages

The user interface publishes:

- `true` or `false` for the light state
- An integer from `1` to `100` for the brightness level

When the MQTT connection is established, the device publishes `online` to the configured state topic. The MQTT last-will message publishes `offline` if the connection is lost unexpectedly.

## Configuration

Update the project configuration before building the firmware. The relevant values include:

- Wi-Fi SSID
- Wi-Fi password
- MQTT broker address
- MQTT client ID
- MQTT username
- MQTT password
- MQTT topic names

These values are defined in the project configuration module. Do not commit real Wi-Fi or MQTT credentials to a public repository.

The ESP32-S3 requires a 2.4 GHz Wi-Fi network. A 5 GHz-only network cannot be used by the device.

## Partition Table

The current firmware image requires an application partition larger than the default 1 MB partition. The project therefore uses a custom partition table with an 8 MB factory application partition.

Example:

```csv
# Name,     Type, SubType, Offset,   Size,     Flags
nvs,        data, nvs,     0x9000,   0x6000,
phy_init,   data, phy,     0xF000,   0x1000,
factory,    app,  factory, 0x10000,  0x800000,
```

The current firmware image uses approximately 7.8 MB of the 8 MB application partition. Further growth may require additional size optimisation or a different flash layout.

## Building

The project uses the ESP32-S3 ESP-IDF Rust target.

Build the release firmware:

```powershell
cargo build --release --target xtensa-esp32s3-espidf
```

## Flashing and Monitoring

Connect the ESP32-S3 to the computer and verify the serial port. The following command flashes the firmware, uses the custom partition table, and starts the serial monitor:

```powershell
cargo espflash flash `
  --release `
  --target xtensa-esp32s3-espidf `
  --partition-table .\partitions.csv `
  --monitor `
  --chip esp32s3 `
  --port COM3
```

Replace `COM3` with the serial port assigned to the device.

## Wi-Fi Recovery Behaviour

The device does not stop when the initial Wi-Fi connection fails. It logs the connection error, waits for the configured retry interval, and tries to connect again.

This allows the controller to start before the network is available or to recover after a temporary access point outage.

## Project Structure

```text
src/
├── bin/
│   └── main.rs
├── lib.rs
├── config.rs
├── cst328.rs
├── mqtt.rs
├── slint_backend.rs
├── st7789.rs
└── wifi.rs
```

The main application coordinates the display, touch input, user interface, Wi-Fi connection, and MQTT communication. The Wi-Fi and MQTT implementations are kept in separate modules.

## Runtime Flow

1. The ESP-IDF runtime and logging system are initialised.
2. ESP32-S3 peripherals, the system event loop, and NVS are acquired.
3. The display and touch controller are initialised.
4. Wi-Fi is started and the device attempts to connect.
5. The device retries the connection if Wi-Fi is unavailable.
6. The Slint user interface is created.
7. The MQTT client connects to the configured broker.
8. MQTT messages are exchanged with the smart home system.
9. The main loop processes touch input, updates the UI, and renders display frames.

## Troubleshooting

### The firmware does not fit the application partition

Use the custom partition table when flashing:

```powershell
--partition-table .\partitions.csv
```

The application partition must be large enough for the generated release image.

### Wi-Fi connection times out

Check the following:

- The SSID and password are correct.
- The access point provides a 2.4 GHz network.
- WPA2-Personal is enabled if the access point does not support the configured security mode.
- The device is within range of the access point.
- MAC filtering is disabled or the ESP32-S3 has been allowed.

The device should continue retrying instead of terminating.

### The display remains dark

Check the display wiring, especially:

- Backlight on GPIO5
- Chip select on GPIO42
- Data/command on GPIO41
- Reset on GPIO39
- SPI wiring and power supply

### Touch input does not work

Check the I²C wiring:

- SDA on GPIO1
- SCL on GPIO3
- Correct power supply
- Correct ground connection

## License

This is a private, proprietary project intended solely for the personal use of the author.

All rights reserved. No permission is granted to copy, reproduce, modify, publish, distribute, sublicense, or use this project or substantial parts of it without the author's prior written permission.

The licensing terms may be changed in the future.
