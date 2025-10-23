#![no_std]
#![no_main]

use esp_idf_sys::*;
use esp_idf_hal::{
    delay::Ets,
    gpio::PinDriver,
    peripherals::Peripherals,
    i2c::{I2cConfig, I2cDriver},
};
use esp_idf_svc::{
    eventloop::EspSystemEventLoop,
    nvs::EspDefaultNvsPartition,
    wifi::{AuthMethod, BlockingWifi, ClientConfiguration, Configuration, EspWifi},
};
use log::{info, error};
use anyhow::Result;
use serde_json::json;
use core::time::Duration;
use embedded_dht_rs::dht22::Dht22;
use alloc::ffi::CString;
extern crate alloc;

#[inline(always)]
fn ms_to_ticks(ms: u32) -> u32 {
    (ms as u64 * configTICK_RATE_HZ as u64 / 1000) as u32
}

struct SimpleMqttClient {
    client: *mut esp_mqtt_client,
}

impl SimpleMqttClient {
    fn new(broker_url: &str, username: &str, password: &str, client_id: &str) -> Result<Self> {
        unsafe {
            let broker_url_cstr = CString::new(broker_url)?;
            let username_cstr = CString::new(username)?;
            let password_cstr = CString::new(password)?;
            let client_id_cstr = CString::new(client_id)?;

            let config = esp_mqtt_client_config_t {
                broker: esp_mqtt_client_config_t_broker_t {
                    address: esp_mqtt_client_config_t_broker_t_address_t {
                        uri: broker_url_cstr.as_ptr(),
                        ..core::mem::zeroed()
                    },
                    ..core::mem::zeroed()
                },
                credentials: esp_mqtt_client_config_t_credentials_t {
                    username: username_cstr.as_ptr(),
                    client_id: client_id_cstr.as_ptr(),
                    authentication: esp_mqtt_client_config_t_credentials_t_authentication_t {
                        password: password_cstr.as_ptr(),
                        ..core::mem::zeroed()
                    },
                    ..core::mem::zeroed()
                },
                ..core::mem::zeroed()
            };

            let client = esp_mqtt_client_init(&config as *const _);
            if client.is_null() {
                return Err(anyhow::anyhow!("Gagal inisialisasi klien MQTT"));
            }

            let err = esp_mqtt_client_start(client);
            if err != ESP_OK {
                return Err(anyhow::anyhow!("Gagal memulai klien MQTT: {}", err));
            }

            vTaskDelay(ms_to_ticks(2000));
            Ok(Self { client })
        }
    }

    fn publish(&self, topic: &str, data: &str, qos: i32) -> Result<()> {
        unsafe {
            let topic_cstr = CString::new(topic)?;
            let msg_id = esp_mqtt_client_publish(
                self.client,
                topic_cstr.as_ptr(),
                data.as_ptr(), // sudah benar *const u8
                data.len() as i32,
                qos,
                0,
            );

            if msg_id < 0 {
                Err(anyhow::anyhow!("Gagal kirim pesan MQTT"))
            } else {
                info!("📡 Telemetri dikirim ID: {}", msg_id);
                Ok(())
            }
        }
    }
}

impl Drop for SimpleMqttClient {
    fn drop(&mut self) {
        unsafe {
            esp_mqtt_client_stop(self.client);
            esp_mqtt_client_destroy(self.client);
            info!("Klien MQTT dihentikan");
        }
    }
}

fn read_ds3231_time(i2c: &mut I2cDriver<'static>) -> Option<(u16, u8, u8, u8, u8, u8)> {
    const DS3231_ADDR: u8 = 0x68;
    let mut data = [0u8; 7];

    if let Err(e) = i2c.write_read(DS3231_ADDR, &[0x00], &mut data, Duration::from_millis(100)) {
        error!("❌ Gagal membaca DS3231: {:?}", e);
        return None;
    }

    fn bcd_to_dec(bcd: u8) -> u8 {
        (bcd >> 4) * 10 + (bcd & 0x0F)
    }

    let second = bcd_to_dec(data[0] & 0x7F);
    let minute = bcd_to_dec(data[1]);
    let hour = bcd_to_dec(data[2] & 0x3F);
    let day = bcd_to_dec(data[4]);
    let month = bcd_to_dec(data[5] & 0x1F);
    let year = 2000 + bcd_to_dec(data[6]) as u16;

    Some((year, month, day, hour, minute, second))
}

#[no_mangle]
fn main() {
    esp_idf_sys::link_patches();
    esp_idf_svc::log::EspLogger::initialize_default();
    info!("🚀 ESP32S3: DHT22 + DS3231 + WiFi + MQTT (3s interval)");

    let peripherals = Peripherals::take().unwrap();
    let sys_loop = EspSystemEventLoop::take().unwrap();
    let nvs = EspDefaultNvsPartition::take().unwrap();

    let mut wifi = BlockingWifi::wrap(
        EspWifi::new(peripherals.modem, sys_loop.clone(), Some(nvs)).unwrap(),
        sys_loop,
    )
    .unwrap();

    if let Err(e) = connect_wifi(&mut wifi) {
        error!("❌ WiFi gagal: {:?}", e);
        return;
    }

    let mqtt_client = match SimpleMqttClient::new(
        "mqtt://mqtt.thingsboard.cloud:1883",
        "reva",
        "reva123",
        "x1a7pbahq1qzv6ei32db",
    ) {
        Ok(c) => c,
        Err(e) => {
            error!("❌ MQTT gagal: {:?}", e);
            return;
        }
    };

    // ✅ Inisialisasi I2C untuk RTC DS3231
    let i2c = I2cDriver::new(
        peripherals.i2c0,
        peripherals.pins.gpio1, // SDA
        peripherals.pins.gpio2, // SCL
        &I2cConfig::default(),
    )
    .unwrap();
    let mut i2c = i2c;

    // ✅ Inisialisasi sensor DHT22
    let pin = PinDriver::input_output_od(peripherals.pins.gpio4).unwrap();
    let delay = Ets;
    let mut dht22 = Dht22::new(pin, delay);

    loop {
        let mut telemetry = json!({});

        // Baca sensor DHT22
        match dht22.read() {
            Ok(reading) => {
                telemetry["temperature"] = json!(reading.temperature);
                telemetry["humidity"] = json!(reading.humidity);
                info!("🌡 Temp: {:.2}°C | 💧 Hum: {:.2}%", reading.temperature, reading.humidity);
            }
            Err(e) => error!("Gagal baca DHT22: {:?}", e),
        }

        // Baca waktu dari DS3231
        if let Some((year, month, day, hour, minute, second)) = read_ds3231_time(&mut i2c) {
            telemetry["date"] = json!(format!("{:04}-{:02}-{:02}", year, month, day));
            telemetry["time"] = json!(format!("{:02}:{:02}:{:02}", hour, minute, second));
            info!("⏰ RTC: {:04}-{:02}-{:02} {:02}:{:02}:{:02}", year, month, day, hour, minute, second);
        } else {
            error!("❌ Gagal baca waktu RTC");
        }

        // Kirim data ke ThingsBoard
        let payload = telemetry.to_string();
        if let Err(e) = mqtt_client.publish("v1/devices/me/telemetry", &payload, 1) {
            error!("❌ Gagal kirim telemetri: {:?}", e);
        } else {
            info!("✅ Telemetri terkirim: {}", payload);
        }

        // ⏱ Delay 3 detik
        unsafe { vTaskDelay(ms_to_ticks(3000)); }
    }
}

fn connect_wifi(wifi: &mut BlockingWifi<EspWifi<'static>>) -> Result<()> {
    let ssid = "revaa";
    let password = "revaaulia123";

    info!("🔌 Menghubungkan WiFi {}", ssid);

    let wifi_config = Configuration::Client(ClientConfiguration {
        ssid: heapless::String::try_from(ssid).unwrap(),
        password: heapless::String::try_from(password).unwrap(),
        auth_method: AuthMethod::WPA2Personal,
        ..Default::default()
    });

    wifi.set_configuration(&wifi_config)?;
    wifi.start()?;
    wifi.connect()?;
    wifi.wait_netif_up()?;

    let ip_info = wifi.wifi().sta_netif().get_ip_info()?;
    info!("📶 Terhubung! IP: {}", ip_info.ip);
    Ok(())
}
