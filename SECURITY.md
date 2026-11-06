# Security policy

## Supported versions

Only the latest 0.2.x release receives fixes.

## Reporting a vulnerability

Please do not open a public issue. Report it privately through GitHub:
[Report a vulnerability](https://github.com/robmeijerink/laposta-rs/security/advisories/new).

Include the version of the crate and, if you can, a few lines of code that reproduce the problem.
Do not send real API keys, webhook secrets or subscriber data.

## What is in scope

This crate handles Laposta API keys and, with the `webhook` feature, webhook secrets, so these are
worth reporting:

- an API key or webhook secret that ends up in a log, an error or `Debug` output;
- a flaw in the webhook signature check that accepts a signature it should reject;
- a request that goes to another host than intended, or one that follows a redirect;
- an id or value that passes validation but should not, or that changes silently on the way to the
  API (for example a path segment that escapes its place in the URL);
- a panic or unbounded memory use caused by a crafted response or webhook body.

## What is not in scope

- Problems in the Laposta API or platform itself. Report those to Laposta.
- Behaviour that is documented: errors can contain a part of a response body, which may hold
  subscriber data, and a response has no size limit. A webhook signature does not protect against
  replays, so check `date_requested` yourself if that matters. The copy of the API key inside a
  request header is held by the HTTP client and is not wiped from memory.
