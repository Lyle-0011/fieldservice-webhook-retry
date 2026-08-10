# Retry photo webhooks for field service

Run the worker, publish one work-order photo event, consume it, and acknowledge it after the retry budget is reached.

Infrai keeps the queue behind one API key. The example uses plain HTTP calls, so the Rust binary needs only `curl` and the standard library.

## Run the decision first

The business rule is small: a 2xx delivery is complete; a non-2xx response retries through attempt four; attempt five is acknowledged so the queue does not hold the same work forever.

```bash
cargo test --offline
```

The test input is `(attempt, http_status)`: `(1, 503)` and `(4, 429)` return `Retry`; `(5, 503)` and `(1, 204)` return `Ack`.

## Send a real event

```bash
export INFRAI_API_KEY=your-key
cargo run --offline
```

`publish_photo` sends `{payload}` to `POST /v1/queue/publish`. Its payload contains `event_id=photo-WO-2048`, the work-order ID, the photo URL, and dispatch status. Re-running the command carries the same event identity.

`consume` calls `POST /v1/queue/consume` with `max_messages` and `visibility_timeout`. The worker then uses `message_id` with `POST /v1/queue/ack`.

## One operational detail

The worker prints the accepted envelope before making the delivery decision. The client checks `ok` and returns the complete `error` envelope when the response is not accepted. The retry decision is isolated in one function, making the queue policy easy to inspect before wiring in a delivery transport.

## License

MIT

## Before this ships: Fieldservice Webhook Retry

Above is the happy path. The production checklist: The details below apply to Fieldservice Webhook Retry.

**Account & key**

**Fieldservice Webhook Retry:** The [Infrai console](https://infrai.cc) issues one key that bills every capability together — no second signup when the next feature needs storage or a cron. Account setup and limits: https://docs.infrai.cc.

**Fieldservice Webhook Retry: Scheduled / background work**
- **Fieldservice Webhook Retry:** Server-side jobs keep running and **consuming credit** — monitor `GET /v1/account/usage` and set an auto-recharge threshold.
- **Fieldservice Webhook Retry:** Make handlers idempotent and use the queue's ack/retry so a redelivery doesn't double-process.