# laposta-rs

Async Rust client for the [Laposta](https://laposta.nl) email marketing API (v2).

> This is an unofficial, community-maintained library. It is not affiliated with, endorsed by or supported by Laposta.

It covers lists, fields, segments, members, campaigns, reports and webhooks, including bulk member sync and cursor based history.

```toml
[dependencies]
laposta-rs = "0.1"
tokio = { version = "1", features = ["macros", "rt-multi-thread"] }
```

## Usage

```rust
use std::net::Ipv4Addr;

use laposta_rs::{Client, Email, ListId, NewMember};

#[tokio::main]
async fn main() -> laposta_rs::Result<()> {
    let client = Client::new(std::env::var("LAPOSTA_API_KEY").unwrap());
    let list_id = ListId::new("BaImMu3JZA");

    let member = client
        .members()
        .create(
            &list_id,
            &NewMember::new(Email::parse("rob@example.net")?, Ipv4Addr::new(198, 51, 100, 10))
                .custom_field("name", "Rob")
                .upsert(true),
        )
        .await?;

    println!("{} is {:?}", member.email, member.state);
    Ok(())
}
```

The IP address is the one the subscriber signed up from; the API requires it.

Find your API key in Laposta under *Access & Subscription* → *Links - API*.

### Types instead of strings

List, member and campaign ids are distinct types, so passing a campaign id where a list id is expected does not compile. `Email`, `Timestamp` and `Limit` are validated when you construct them; `std::net::IpAddr` and `url::Url` are used where the API expects an address or URL. Members can be addressed by id or email through `MemberRef`.

Fields that only make sense together are modelled that way: `NewFieldKind::SelectSingle` carries its options, `NewCampaign::new` needs at least one audience, and `SyncMembers::new` needs at least one action.

### Paginating

`members().list` returns a single `Page`. To walk through everything, use the stream variants, which follow the cursors for you:

```rust
use futures::TryStreamExt;
use laposta_rs::MemberQuery;

let mut members = client.members().stream(&list_id, MemberQuery::new());
while let Some(member) = members.try_next().await? {
    println!("{}", member.email);
}
```

The same works for `members().history_stream` and `campaigns().history_stream`.

### Errors

Failed requests return `Error::Api` with the HTTP status, error type, code and offending parameter:

```rust
match client.lists().get(&list_id).await {
    Err(e) if e.is_not_found() => println!("no such list"),
    Err(e) => return Err(e),
    Ok(list) => println!("{}", list.name),
}
```

### Rate limiting

Laposta allows 30 requests per minute on free and 120 on paid accounts. Retrying after an HTTP 429 is opt-in:

```rust
let client = Client::builder().api_key(key).max_retries(3).build()?;
```

The client then waits for the `Retry-After` duration (at most 60 seconds) before each retry.

### Timeouts and redirects

Requests time out after 60 seconds (10 seconds to connect); change it with `Client::builder().timeout(..)`. Large bulk syncs may need more. The default HTTP client does not follow redirects, since the API never redirects. If you pass your own `reqwest::Client`, its settings apply as is.

The base URL must use `https`. Plain `http` is only accepted for loopback addresses, which is useful for mock servers.

### Webhooks

With the `webhook` feature (on by default) you can verify and parse incoming notifications:

```rust
use laposta_rs::webhooks::{parse_payload, verify_signature};

if verify_signature(body, signature_header, webhook.secret.as_ref().unwrap().expose()) {
    let payload = parse_payload(body)?;
}
```

A signature does not protect against replays; compare `payload.date_requested` with the current time if that matters.

## Features

| Feature | Default | |
|---|---|---|
| `rustls-tls` | yes | TLS through rustls |
| `native-tls` | no | TLS through the platform library |
| `webhook` | yes | Signature verification and payload types |

Use `default-features = false, features = ["native-tls"]` to switch TLS backends.

## Testing

```sh
cargo test
```

The unit and integration tests run against a mock server. To exercise the real API with a throwaway list:

```sh
LAPOSTA_API_KEY=... cargo test --test live -- --ignored
```

## License

MIT

---

This library is developed by [Solvalutions](https://solvalutions.nl) and [Rob Meijerink](https://robmeijerink.nl).

*&copy;&nbsp;2026 &mdash; Built by Rob Meijerink.*
