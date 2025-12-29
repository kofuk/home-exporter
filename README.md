# SwitchBot Meter BLE Thermo-Hygro Collector

This is a MicroPython application for Raspberry Pi Pico 2 W that collects temperature and humidity data from SwitchBot Meter
and sends the data to Prometheus using Prometheus Remote Write protocol over HTTPS.

## Setup

0. Install MicroPython on your Raspberry Pi Pico 2 W.
1. Create config.json based on config.sample.json.
2. Convert required CA certificates into DER format and save as ca.der. (guide below)
3. Deploy all files
   ```bash
   $ mise run deploy
   ```

###  How to convert CA certificates to DER format

1. Find required certificates by running the following command:
   ```bash
   openssl s_client -showcerts -connect ${your_server}:443 </dev/null
   ```
   Required certificates are ones with "CA" in the subject field.
2. Copy each certificate (including `-----BEGIN CERTIFICATE-----` and `-----END CERTIFICATE-----`) into a file (e.g. `ca.pem`).
3. Convert the PEM file into DER format:
   ```bash
   openssl x509 -in ca.pem -outform der -out ca.der
   ```

## Reference 

- https://github.com/OpenWonderLabs/SwitchBotAPI-BLE/blob/latest/devicetypes/meter.md#new-broadcast-message
