import asyncio
import json
import aioble
from machine import WDT
import network
from prometheus_remote_write_payload import PrometheusRemoteWritePayload
import ntptime
import time
import ssl
import ubinascii

class PrometheusConfig:
    def __init__(self, endpoint: str, username: str, password: str, location: str):
        self.endpoint = endpoint
        self.username = username
        self.password = password
        self.location = location

class Config:
    def __init__(self, mac_addr: str, ssid: str, password: str, prometheusEndpoint: str, prometheusUsername: str, prometheusPassword: str, prometheusLocation: str):
        self.mac_addr = mac_addr if mac_addr != '' else None
        self.ssid = ssid
        self.password = password
        self.prometheus = PrometheusConfig(prometheusEndpoint, prometheusUsername, prometheusPassword, prometheusLocation)

class ThermoHygroData:
    def __init__(self, temperature: float, humidity: int):
        self.temperature = temperature
        self.humidity = humidity

def load_config() -> Config:
    with open('config.json', 'r') as f:
        config_data = json.load(f)
    return Config(
        mac_addr=config_data['hwMac'],
        ssid=config_data['wifi']['ssid'],
        password=config_data['wifi']['password'],
        prometheusEndpoint=config_data['prometheus']['remoteWriteEndpoint'],
        prometheusUsername=config_data['prometheus']['username'],
        prometheusPassword=config_data['prometheus']['password'],
        prometheusLocation=config_data['prometheus']['location']
    )

class WatchdogWrapper:
    def __init__(self, timeout: int):
        self.wdt = WDT(timeout=timeout)

    def feed(self):
        self.wdt.feed()

# Because of incomplete support for async generators in MicroPython, we have to use callbacks.
async def scan(config: Config, callback, wd: WatchdogWrapper):
    async with aioble.scan(
        duration_ms=0,
        interval_us=10_000,
        window_us=3_000_000,
        active=False
    ) as scanner:
        async for result in scanner:
            if wd:
                wd.feed()

            addr = result.device.addr_hex()
            rssi = result.rssi
            local_name = result.name()

            manufacturer_data = list(result.manufacturer())
            if not manufacturer_data:
                continue

            company_id, data = manufacturer_data[0]

            if not config.mac_addr:
                print(f"device detected: addr={addr}, rssi={rssi}, "
                      f"localName={local_name}, companyId={company_id}")
            else:
                if addr.lower() != config.mac_addr.lower():
                    continue

            if len(data) < 11:
                continue

            temp_decimal_byte = data[8]
            temp_integer_byte = data[9]
            humidity_byte = data[10]

            # Temperature
            temperature_decimal = temp_decimal_byte & 0x0F
            temperature_positive = (temp_integer_byte & 0x80) != 0
            temperature_integer = temp_integer_byte & 0x7F

            temperature = float(temperature_integer) + float(temperature_decimal) / 10.0
            if not temperature_positive:
                temperature = -temperature

            # Humidity
            humidity = humidity_byte & 0x7F

            await callback(ThermoHygroData(temperature=temperature, humidity=humidity))

class HTTPClient:
    def __init__(self, wdt: WatchdogWrapper):
        context = ssl.SSLContext(ssl.PROTOCOL_TLS_CLIENT)
        context.verify_mode = ssl.CERT_REQUIRED
        context.check_hostname = True
        with open('/ca.der', 'rb') as f:
            ca_cert = f.read()
        context.load_verify_locations(cadata=ca_cert)
        self.ssl_context = context
        self.wdt = wdt

    def _parse_url(self, url: str) -> tuple[str, str]:
        if url.startswith('https://'):
            url = url[len('https://'):]
        elif url.startswith('http://'):
            url = url[len('http://'):]

        parts = url.split('/', 1)
        host = parts[0]
        path = '/' + parts[1] if len(parts) > 1 else '/'

        return host, path

    async def post(self, url: str, auth: tuple[str, str]|None, headers: dict[str, str], data: bytes):
        if auth:
            user_pass = f"{auth[0]}:{auth[1]}"
            encoded = ubinascii.b2a_base64(user_pass.encode()).decode().strip()
            headers['Authorization'] = f"Basic {encoded}"

        host, path = self._parse_url(url)

        reader, writer = await asyncio.open_connection(
            host=host,
            port=443,
            ssl=self.ssl_context
        )

        request_line = f"POST {path} HTTP/1.1\r\n"
        headers['Content-Length'] = str(len(data))
        # This is important because we read until EOF
        headers['Connection'] = 'close'
        headers['Host'] = host
        header_lines = '\r\n'.join(f"{key}: {value}" for key, value in headers.items())
        full_request = (
            request_line +
            header_lines +
            "\r\n\r\n"
        ).encode() + data

        self.wdt.feed()
        writer.write(full_request)
        await writer.drain()

        self.wdt.feed()
        await reader.read(-1)

        writer.close()
        await writer.wait_closed()

class MetricsExporter:
    def __init__(self, config: PrometheusConfig, wdt: WatchdogWrapper):
        self.config = config
        self.last_sent_time = 0
        self.http_client = HTTPClient(wdt)

    async def export(self, data: ThermoHygroData):
        if self.last_sent_time != 0 and (time.time() - self.last_sent_time) < 10:
            return

        self.last_sent_time = time.time()

        payload = PrometheusRemoteWritePayload()
        payload.add_data(
            "thermohydro_temperature_celsius",
            {"location": self.config.location},
            data.temperature,
            int(time.time() * 1000)
        )
        payload.add_data(
            "thermohydro_humidity_percent",
            {"location": self.config.location},
            float(data.humidity),
            int(time.time() * 1000)
        )

        await self.http_client.post(
            url=self.config.endpoint,
            auth=(self.config.username, self.config.password),
            headers={
                'Content-Type': 'application/x-protobuf',
                'Content-Encoding': 'snappy',
                'User-Agent': 'thermo-hygro-collector/0.0.0',
                'X-Prometheus-Remote-Write-Version': '0.1.0'
            },
            data=payload.get_payload()
        )

async def main():
    wdt = WatchdogWrapper(timeout=8388)

    config = load_config()

    wlan = network.WLAN(network.STA_IF)
    wlan.active(True)
    wlan.connect(config.ssid, config.password)

    for _ in range(20):
        if wlan.isconnected():
            break
        await asyncio.sleep(1)

        wdt.feed()

    if wlan.isconnected():
        print('Connected to Wi-Fi')

    ntptime.settime()

    exporter = MetricsExporter(config.prometheus, wdt)

    async def callback(data: ThermoHygroData):
        print(f"temperature={data.temperature:.1f}C, humidity={data.humidity}%")
        await exporter.export(data)

    await scan(config, callback, wdt)

if __name__ == '__main__':
    asyncio.run(main())
