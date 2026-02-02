pub struct MetricsRepository {
    temperature: Option<f32>,
    humidity: Option<u8>,
}

impl MetricsRepository {
    pub fn new() -> Self {
        Self {
            temperature: None,
            humidity: None,
        }
    }

    pub fn get_temperature(self) -> Option<f32> {
        self.temperature
    }

    pub fn set_temperature(&mut self, temperature: f32) {
        self.temperature = Some(temperature);
    }

    pub fn get_humidity(self) -> Option<u8> {
        self.humidity
    }

    pub fn set_humidity(&mut self, humidity: u8) {
        self.humidity = Some(humidity);
    }
}
