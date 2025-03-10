# ESP32 Quiz Device

A portable keychain-sized quiz device built with ESP32, featuring an LCD display and a 3D printed case. This device is designed as a conference tool for participants to engage in quizzes during events. The codebase also serves as an example for conference workshops.

## Features

- Collects single choice answers from participants
- Transmits responses via MQTT protocol
- Works with a separate Python backend to determine winners

## Requirements

- ESP32 development board
- LCD display
- Docker (for development environment)
- VS Code or other IDE with Dev Container support
- Chrome-based browser (for macOS users)

## Environment Variables

The following environment variables must be set to build and run the project:

```
WIFI_SSID
WIFI_PASSWORD
MQTT_BROKER_URL
MQTT_USER
MQTT_PASSWORD
```

### Example Build

```bash
# Set required environment variables
export WIFI_SSID="your_wifi_ssid"
export WIFI_PASSWORD="your_wifi_password"
export MQTT_BROKER_URL="your_mqtt_broker_url"
export MQTT_USER="your_mqtt_username"
export MQTT_PASSWORD="your_mqtt_password"

# Build the project
cargo build
```

Note: All environment variables must be set before building, otherwise you'll encounter compilation errors. The project uses the `env!` macro which requires these variables to be defined at compile time.

## Development Setup

This project supports development containers for consistent development environments across different systems.

### Using Dev Containers

1. Choose the appropriate dev container for your system:
   - `.devcontainer/linux/devcontainer.json` for Linux
   - `.devcontainer/mac/devcontainer.json` for macOS
   - `.devcontainer/wsl2/devcontainer.json` for Windows WSL2

2. Open the project in VS Code or another IDE that supports Dev Containers.

3. When opening for the first time, the container will build automatically. This process may take several minutes.

### macOS USB Port Forwarding

On macOS, USB ports are not forwarded to the container by default. You'll need to use Web USB:

1. The container will start a web page for flashing
2. Open this page in a Chrome-based browser (which supports Web USB)
3. Use the following command to flash the device:

```
web-flash --chip=esp32 target/xtensa-esp32-espidf/debug/esp32-mqtt
```

