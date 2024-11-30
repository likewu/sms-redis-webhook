## sms-redis-webhook that persists json in Redis

### Motivation

As of now, this project is a simple webhook that persists all messages in a Redis instance in plain text (no attachment/media support).

### Install

Rename `.env.template` to `.env` and fill in the values. There's a Dockerfile if you need to spin a Redis instance for dev,

```sh
./docker-compose up -d --build
```

should get that covered.

Run the project with

```sh
cargo run
```

### Test in dev

```sh

curl 'http://localhost:{PORT}/webhook?token={PRIVATE_EXCHANGE_TOKEN}&key=auditlog' \
-H 'Content-Type: application/json' \
-d '{"msgtype": "text",
     "text": {
          "content": "钉钉机器人群消息测试"
     }
   }'
```