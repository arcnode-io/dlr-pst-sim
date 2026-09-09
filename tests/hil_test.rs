//! Hardware-in-Loop tests for ESP32 firmware.
//!
//! Tests include:
//! - MQTT integration: temperature sensor publishes to MQTT broker
//! - Tap control: device reacts to a published DLR rating with a tap decision

mod fixtures;

use fixtures::build::{RELEASE_BIN_PATH, build_release_firmware};
use fixtures::containers::{Container, start_mqtt_broker};
use fixtures::flash::download_and_reset;
use rumqttc::v5::{
    AsyncClient, Event, EventLoop, MqttOptions,
    mqttbytes::{QoS, v5::Packet},
};
use std::error::Error;
use std::time::{Duration, Instant};
use tokio::time::timeout;

/// Topic where temperature readings are published.
const TEMP_TOPIC: &str = "test/temp/F";

/// Topic the test publishes a synthetic DLR rating to.
const RATING_TOPIC: &str = "test/line_rating/A";

/// Topic the device publishes its tap-position decision to.
const TAP_POSITION_TOPIC: &str = "test/tap_position";

/// A rating that unambiguously targets Tap1 in `tap_control`'s bands, so a
/// fresh device (starting at Tap4) takes exactly one step to Tap3.
const HIGH_RATING_A: &str = "2500.0";

/// Maximum time to wait for MQTT message (in seconds).
const MQTT_TIMEOUT_SECS: u64 = 80;

/// Starts a broker, checks `WIFI_PASSWORD` is set, and builds the release
/// firmware against the broker's port.
async fn arrange_broker_and_firmware(start: Instant) -> Result<Container, Box<dyn Error>> {
    let broker = start_mqtt_broker().await?;
    println!(
        "[{:.1}s] broker on port {}",
        start.elapsed().as_secs_f32(),
        broker.port
    );

    if std::env::var("WIFI_PASSWORD").is_err() {
        return Err("WIFI_PASSWORD environment variable must be set".into());
    }

    build_release_firmware(broker.port)?;
    println!("[{:.1}s] firmware built", start.elapsed().as_secs_f32());
    Ok(broker)
}

/// Flashes the release binary to the device and resets it.
fn flash_firmware(start: Instant) -> Result<(), Box<dyn Error>> {
    download_and_reset(RELEASE_BIN_PATH).map_err(|e| -> Box<dyn Error> { e.into() })?;
    println!(
        "[{:.1}s] firmware flashed + device reset",
        start.elapsed().as_secs_f32()
    );
    Ok(())
}

/// Waits for a single Publish packet on `eventloop` and returns its payload
/// as a UTF-8 string, or an error on timeout / decode failure.
async fn wait_for_publish_payload(
    eventloop: &mut EventLoop,
    start: Instant,
) -> Result<String, Box<dyn Error>> {
    let result = timeout(Duration::from_secs(MQTT_TIMEOUT_SECS), async {
        loop {
            let notification = eventloop.poll().await?;
            if let Event::Incoming(Packet::Publish(p)) = notification {
                let payload = String::from_utf8(p.payload.to_vec())
                    .map_err(|_| "Invalid UTF-8 in payload")?;
                println!(
                    "[{:.1}s] received: {}",
                    start.elapsed().as_secs_f32(),
                    payload
                );
                return Ok::<String, Box<dyn Error + Send + Sync>>(payload);
            }
        }
    })
    .await;

    match result {
        Ok(Ok(payload)) => Ok(payload),
        Ok(Err(e)) => Err(e),
        Err(_) => Err(format!("Test timed out after {MQTT_TIMEOUT_SECS} seconds").into()),
    }
}

#[tokio::test]
async fn test_hil_mqtt_integration() -> Result<(), Box<dyn Error>> {
    let start = Instant::now();

    // Arrange
    let broker = arrange_broker_and_firmware(start).await?;
    let mqttoptions = MqttOptions::new("hil_test_client", "localhost", broker.port);
    let (client, mut eventloop) = AsyncClient::new(mqttoptions, 10);
    // Subscribe BEFORE flashing so we don't miss the first message.
    client.subscribe(TEMP_TOPIC, QoS::AtLeastOnce).await?;

    // Act
    flash_firmware(start)?;
    let payload = wait_for_publish_payload(&mut eventloop, start).await?;

    // Assert
    let temp_f: f32 = payload
        .parse()
        .map_err(|_| "Payload is not a valid float")?;
    assert!(
        (50.0..=104.0).contains(&temp_f),
        "Temperature {temp_f} is outside reasonable range"
    );
    println!(
        "[{:.1}s] SUCCESS: valid temperature {}F",
        start.elapsed().as_secs_f32(),
        temp_f
    );
    Ok(())
}

#[tokio::test]
async fn test_hil_tap_position_reacts_to_rating() -> Result<(), Box<dyn Error>> {
    let start = Instant::now();

    // Arrange
    let broker = arrange_broker_and_firmware(start).await?;
    let mqttoptions = MqttOptions::new("hil_tap_test_client", "localhost", broker.port);
    let (client, mut eventloop) = AsyncClient::new(mqttoptions, 10);
    // Publish the synthetic rating retained, BEFORE the device subscribes --
    // retain guarantees delivery regardless of subscribe-vs-publish ordering.
    client
        .publish(RATING_TOPIC, QoS::AtLeastOnce, true, HIGH_RATING_A)
        .await?;
    client
        .subscribe(TAP_POSITION_TOPIC, QoS::AtLeastOnce)
        .await?;

    // Act
    flash_firmware(start)?;
    let payload = wait_for_publish_payload(&mut eventloop, start).await?;

    // Assert -- first tick only steps one position from the conservative
    // start (Tap4) toward the target the rating implies (Tap1).
    assert_eq!(payload, "TAP_3", "expected one step from Tap4 toward Tap1");
    println!(
        "[{:.1}s] SUCCESS: device reacted with {}",
        start.elapsed().as_secs_f32(),
        payload
    );
    Ok(())
}
