# Changelog

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
