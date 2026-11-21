# Changelog

## 1.0.2

- Fixed decoding of member and campaign history: the API wraps each event as `{"event": {...}}`, which resulted in empty events. Both the wrapped and the bare form are accepted now.
- Fixed `fields().all()` and `get()` failing on the email field of a list, for which the API returns `custom_name: null`. `Field::custom_name` is an empty string in that case.
- Documented that the members list only accepts `order` together with `order_by`, and fixed the `pagination` example accordingly.
- Documented the environment variables used by the examples.

## 1.0.1

- Added runnable examples for each part of the API in the `examples` directory.

## 1.0.0

First stable release. The API follows semantic versioning; see the stability policy in the README.

- Breaking: `Error::Http` now holds the new `HttpError` instead of `reqwest::Error`. It offers `is_timeout()` and `is_connect()`; the original error is available through `source()`.
- Documented the stability policy in the README.

## 0.2.1

Avoid cloning large response bodies when decoding, which mainly benefits bulk member sync and history requests.

## 0.2.0

Update minimum Rust version to 1.88

## 0.1.1

No changes to the library. First release published through Trusted Publishing.

## 0.1.0

Initial release.
