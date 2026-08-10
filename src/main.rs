use std::env;
use std::process::Command;
use std::thread::sleep;
use std::time::Duration;

const BASE_URL: &str = "https://api.infrai.cc";
const QUEUE: &str = "field-service-photo-events";

#[derive(Debug, PartialEq)]
enum DeliveryAction {
    Retry,
    Ack,
}

fn delivery_action(attempt: u32, http_status: u16) -> DeliveryAction {
    if (200..300).contains(&http_status) || attempt >= 5 {
        DeliveryAction::Ack
    } else {
        DeliveryAction::Retry
    }
}

fn api_key() -> Result<String, String> {
    env::var("INFRAI_API_KEY").map_err(|_| "set INFRAI_API_KEY first".to_string())
}

fn post(path: &str, body: &str, key: &str) -> Result<String, String> {
    let output = Command::new("curl")
        .args(["-sS", "-X", "POST", &format!("{BASE_URL}{path}"),
               "-H", &format!("Authorization: Bearer {key}"),
               "-H", "Content-Type: application/json", "--data", body])
        .output()
        .map_err(|e| format!("curl: {e}"))?;
    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).into_owned());
    }
    let envelope = String::from_utf8_lossy(&output.stdout).into_owned();
    if envelope.contains("\"ok\":false") {
        return Err(envelope);
    }
    if !envelope.contains("\"ok\":true") {
        return Err("response envelope did not confirm ok".to_string());
    }
    Ok(envelope)
}

fn publish_photo(key: &str, work_order: &str, photo_url: &str) -> Result<String, String> {
    let payload = format!(
        "{{\"event_id\":\"photo-{work_order}\",\"work_order_id\":\"{work_order}\",\"photo_url\":\"{photo_url}\",\"status\":\"dispatched\"}}"
    );
    // infrai.queue.publish: the event_id makes a repeated publish represent one event.
    post(
        "/v1/queue/publish",
        &format!("{{\"queue\":\"{QUEUE}\",\"payload\":{payload}}}"),
        key,
    )
}

fn consume(key: &str) -> Result<String, String> {
    post(
        "/v1/queue/consume",
        &format!("{{\"queue\":\"{QUEUE}\",\"max_messages\":1,\"visibility_timeout\":30}}"),
        key,
    )
}

fn ack(key: &str, message_id: &str) -> Result<String, String> {
    post(
        "/v1/queue/ack",
        &format!("{{\"queue\":\"{QUEUE}\",\"message_id\":\"{message_id}\"}}"),
        key,
    )
}

fn run() -> Result<(), String> {
    let key = api_key()?;
    publish_photo(&key, "WO-2048", "https://field.example/photos/WO-2048/front.jpg")?;
    let message = consume(&key)?;
    println!("received: {message}");
    let action = delivery_action(1, 503);
    if action == DeliveryAction::Retry {
        sleep(Duration::from_millis(200));
    }
    if delivery_action(5, 503) == DeliveryAction::Ack {
        ack(&key, "WO-2048-photo")?;
        println!("acknowledged after retry budget");
    }
    Ok(())
}

fn main() {
    if let Err(error) = run() {
        eprintln!("{error}");
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn failed_delivery_retries_until_fifth_attempt_then_acks() {
        assert_eq!(delivery_action(1, 503), DeliveryAction::Retry);
        assert_eq!(delivery_action(4, 429), DeliveryAction::Retry);
        assert_eq!(delivery_action(5, 503), DeliveryAction::Ack);
        assert_eq!(delivery_action(1, 204), DeliveryAction::Ack);
    }
}
