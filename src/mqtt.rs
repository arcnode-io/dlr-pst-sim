//! MQTT client for publishing sensor data.
//!
//! Uses rust-mqtt for no_std async MQTT v5 client.

/// MQTT topic for publishing temperature readings.
pub const MQTT_TOPIC: &str = "test/temp/F";

/// MQTT topic to subscribe to for the DLR dynamic line rating (amps).
///
/// Matches `dlr-operating-envelope/src/mqtt.py::MQTT_TOPIC` as it is
/// published today. Provisional: both repos' readmes document the
/// ADR-002 (`ems/topic_structure_adr.md`) contract
/// (`sites/{site_id}/devices/{device_id}/measurements/dynamic_rating/amps`,
/// `FloatSample {ts, value}`), but neither side's code implements it yet.
/// Migrating this topic name is out of scope here -- tracked as a follow-up.
pub const RATING_TOPIC: &str = "test/line_rating/A";

/// MQTT topic for publishing the tap-position decision. Same provisional
/// convention as `RATING_TOPIC` -- see its docs.
pub const TAP_POSITION_TOPIC: &str = "test/tap_position";

#[cfg(feature = "rust-mqtt")]
use core::fmt::Write;
#[cfg(feature = "rust-mqtt")]
use embassy_net::{Ipv4Address, Stack, tcp::TcpSocket};
#[cfg(feature = "rust-mqtt")]
use embassy_time::Duration;
#[cfg(feature = "rust-mqtt")]
use heapless::String;
#[cfg(feature = "rust-mqtt")]
use rust_mqtt::{
    client::{client::MqttClient, client_config::ClientConfig},
    packet::v5::publish_packet::QualityOfService,
    utils::rng_generator::CountingRng,
};

/// MQTT client for publishing sensor data.
#[cfg(all(feature = "rust-mqtt", feature = "esp-wifi"))]
pub struct Mqtt<'a> {
    /// MQTT client instance
    client: MqttClient<'a, TcpSocket<'a>, 5, CountingRng>,
}

#[cfg(all(feature = "rust-mqtt", feature = "esp-wifi"))]
impl<'a> Mqtt<'a> {
    /// Initialize MQTT client and connect to broker.
    ///
    /// # Arguments
    /// * `stack` - Embassy network stack for TCP/IP connectivity
    ///
    /// # Returns
    /// Connected MQTT client ready for publishing
    pub async fn init(stack: &'a Stack<'a>) -> Self {
        // Parse broker config from env vars baked at build time
        let mqtt_host = env!("MQTT_HOST");
        let mqtt_port: u16 = env!("MQTT_PORT").parse().unwrap();

        #[cfg(feature = "defmt")]
        defmt::info!("🚀 Initializing MQTT client...");
        #[cfg(feature = "defmt")]
        defmt::info!("🏠 MQTT broker: {}:{}", mqtt_host, mqtt_port);

        // Create TCP socket buffers (static for 'static lifetime)
        static mut RX_BUFFER: [u8; 4096] = [0; 4096];
        static mut TX_BUFFER: [u8; 4096] = [0; 4096];

        let mut socket = unsafe {
            let rx_buf = &mut *core::ptr::addr_of_mut!(RX_BUFFER);
            let tx_buf = &mut *core::ptr::addr_of_mut!(TX_BUFFER);
            TcpSocket::new(*stack, rx_buf, tx_buf)
        };

        socket.set_timeout(Some(Duration::from_secs(10)));

        // Connect to MQTT broker
        let ip: Ipv4Address = mqtt_host.parse().unwrap();
        let remote_endpoint = (ip, mqtt_port);

        #[cfg(feature = "defmt")]
        defmt::info!(
            "🔗 Attempting TCP connection to MQTT broker at {}:{}",
            mqtt_host,
            mqtt_port
        );
        #[cfg(feature = "defmt")]
        defmt::info!("⏱️  TCP timeout set to 10 seconds");

        socket.connect(remote_endpoint).await.unwrap();

        #[cfg(feature = "defmt")]
        defmt::info!("✅ TCP connected to MQTT broker successfully!");

        // Create MQTT client config
        let mut config = ClientConfig::new(
            rust_mqtt::client::client_config::MqttVersion::MQTTv5,
            CountingRng(20000),
        );
        config.add_client_id("esp32c3");
        config.max_packet_size = 100;

        // Create MQTT client buffers (static for 'static lifetime)
        static mut MQTT_RECV_BUFFER: [u8; 256] = [0; 256];
        static mut MQTT_SEND_BUFFER: [u8; 256] = [0; 256];

        let mut client = unsafe {
            let send_buf = &mut *core::ptr::addr_of_mut!(MQTT_SEND_BUFFER);
            let recv_buf = &mut *core::ptr::addr_of_mut!(MQTT_RECV_BUFFER);
            MqttClient::<_, 5, _>::new(socket, send_buf, 256, recv_buf, 256, config)
        };

        #[cfg(feature = "defmt")]
        defmt::info!("🤝 Attempting MQTT broker handshake...");
        #[cfg(feature = "defmt")]
        defmt::info!("🆔 Client ID: esp32c3");

        client.connect_to_broker().await.unwrap();

        #[cfg(feature = "defmt")]
        defmt::info!("✅ MQTT broker connected successfully!");

        Self { client }
    }

    /// Publish a temperature value to the specified topic.
    ///
    /// # Arguments
    /// * `topic` - MQTT topic to publish to
    /// * `value` - Temperature value to publish
    pub async fn publish(&mut self, topic: &str, value: f64) {
        let mut payload: String<32> = String::new();
        write!(&mut payload, "{:.2}", value).ok();

        #[cfg(feature = "defmt")]
        defmt::info!(
            "📤 Publishing to topic '{}': {} bytes",
            topic,
            payload.len()
        );
        #[cfg(feature = "defmt")]
        defmt::info!("📄 Payload: '{}'", payload.as_str());

        match self
            .client
            .send_message(topic, payload.as_bytes(), QualityOfService::QoS0, false)
            .await
        {
            Ok(_) => {
                #[cfg(feature = "defmt")]
                defmt::info!("✅ Message published successfully");
            }
            Err(e) => {
                #[cfg(feature = "defmt")]
                defmt::info!("❌ Failed to publish message: {:?}", e);
            }
        }
    }

    /// Publish a string value (e.g. an EnumSample label) to the specified topic.
    ///
    /// # Arguments
    /// * `topic` - MQTT topic to publish to
    /// * `value` - String payload to publish verbatim
    pub async fn publish_str(&mut self, topic: &str, value: &str) {
        #[cfg(feature = "defmt")]
        defmt::info!("📤 Publishing to topic '{}': '{}'", topic, value);

        match self
            .client
            .send_message(topic, value.as_bytes(), QualityOfService::QoS0, false)
            .await
        {
            Ok(_) => {
                #[cfg(feature = "defmt")]
                defmt::info!("✅ Message published successfully");
            }
            Err(e) => {
                #[cfg(feature = "defmt")]
                defmt::info!("❌ Failed to publish message: {:?}", e);
            }
        }
    }

    /// Subscribe to a topic. Call once after `init()`, before the first
    /// `try_receive_rating` call.
    ///
    /// # Arguments
    /// * `topic` - MQTT topic to subscribe to
    pub async fn subscribe(&mut self, topic: &str) {
        match self.client.subscribe_to_topic(topic).await {
            Ok(()) => {
                #[cfg(feature = "defmt")]
                defmt::info!("✅ Subscribed to topic '{}'", topic);
            }
            Err(e) => {
                #[cfg(feature = "defmt")]
                defmt::info!("❌ Failed to subscribe to '{}': {:?}", topic, e);
            }
        }
    }

    /// Poll for a pending subscribed message without blocking the caller.
    ///
    /// # Returns
    /// The parsed numeric payload when a message is ready and decodes as a
    /// float; `None` when nothing is pending, the payload isn't valid UTF-8,
    /// or it doesn't parse as a number.
    pub async fn try_receive_rating(&mut self) -> Option<f64> {
        let (_topic, payload) = match self.client.receive_message_if_ready().await {
            Ok(Some(msg)) => msg,
            Ok(None) => return None,
            Err(e) => {
                #[cfg(feature = "defmt")]
                defmt::info!("❌ Failed to poll for subscribed message: {:?}", e);
                return None;
            }
        };

        core::str::from_utf8(payload).ok()?.trim().parse().ok()
    }
}
