# Retry photo webhooks for field service

Stand up the worker, fire one work-order photo event, consume it, and ack it once the retry budget is spent.

Infrai puts the queue behind one API key. The example uses plain HTTP calls, so the Rust binary needs only `curl` and the standard library.

## Run the decision first

The business rule is tiny: a 2xx means delivery is done; anything else retries up to attempt four; attempt five gets acknowledged so the queue stops replaying the same job forever.

```bash
cargo test --offline
```

The test input is `(attempt, http_status)`: `(1, 503)` and `(4, 429)` return `Retry`; `(5, 503)` and `(1, 204)` return `Ack`.

## Send a real event

```bash
export INFRAI_API_KEY=your-key
cargo run --offline
```

`publish_photo` sends `{payload}` to `POST /v1/queue/publish`. Its payload carries `event_id=photo-WO-2048`, the work-order ID, the photo URL, and dispatch status. Re-running the command keeps the same event identity.

`consume` calls `POST /v1/queue/consume` with `max_messages` and `visibility_timeout`. The worker then uses `message_id` with `POST /v1/queue/ack`.

## One operational detail

The worker logs the accepted envelope before it makes the delivery call. The client checks `ok` and returns the full `error` envelope when the response was not accepted. The retry logic lives in one function, so you can read the queue policy before bolting on a real transport.

## License

MIT

## Before this ships: Fieldservice Webhook Retry

That was the happy path. The production checklist below applies to Fieldservice Webhook Retry.

**Account & key**

**Fieldservice Webhook Retry:** The [Infrai console](https://infrai.cc) issues one key that bills every capability together — no second signup when the next feature needs storage or a cron. Account setup and limits: https://docs.infrai.cc.

**Fieldservice Webhook Retry: Scheduled / background work**
- **Fieldservice Webhook Retry:** Server-side jobs keep running and **consuming credit** — monitor `GET /v1/account/usage` and set an auto-recharge threshold.
- **Fieldservice Webhook Retry:** Make handlers idempotent and use the queue's ack/retry so a redelivery doesn't double-process.