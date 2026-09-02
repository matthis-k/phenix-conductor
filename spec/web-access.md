# Web access

status: specification-only

## Goal

Define a safe default web retrieval contract for agents without making a browser, search provider, or unrestricted HTTP client part of core.

Web content is external data. Retrieval never turns remote text into privileged instructions.

## Components

The default web package may provide two independent interfaces:

- search
- fetch

Search provider integrations remain replaceable plugins. Fetch is ordinary bounded HTTP retrieval.

A browser or computer-use implementation is a separate plugin with a stronger authority contract.

## Fetch defaults

- HTTPS only.
- `GET` and `HEAD` only.
- No cookies.
- No ambient authentication.
- No client certificates.
- No JavaScript execution.
- Redirects are bounded and revalidated at every hop.
- Response body size is bounded before buffering.
- Connect and total request time are bounded.
- Private, loopback, link-local, and metadata-service destinations are denied unless explicitly granted.
- Network authority is checked against the final resolved destination, not only the original hostname.

Suggested portable limits:

- redirects: 5
- buffered response body: 10 MiB
- search results returned to the model: 10

Backends may enforce stricter limits.

## Authority

Web access requires network authority in addition to tool visibility.

Authority should support destination scoping. At minimum the policy must distinguish ordinary public web access from private or local network access.

A redirect cannot expand authority. Every redirect target is checked as a new destination.

Credentials use the common secret-reference contract. A fetch request gets no secret merely because the destination matches a provider that has credentials configured.

## Request

A portable fetch request contains:

- parsed URL
- method
- accepted content classes when needed
- optional explicit headers from an allowlisted portable set

Headers that can carry credentials or alter proxy behavior require a stronger contract. Generic callers do not set `Authorization`, `Cookie`, `Proxy-Authorization`, or arbitrary host routing headers.

## Response

A fetch response contains:

- final URL
- redirect chain
- status
- selected safe response headers
- content type
- content length when known
- bounded body or artifact reference
- retrieval timestamp
- truncation state

Large or binary responses should move into the artifact layer instead of becoming an unbounded model message.

## Search

A search request contains:

- query
- result limit
- optional freshness or domain constraints supported by the selected provider

A portable search result contains:

- title
- URL
- provider snippet when present
- rank
- provider-neutral publication or update time when known

Search snippets are discovery metadata. The agent fetches a result when it needs source content.

Search providers declare their own network and credential requirements.

## Content handling

Fetched content enters prompt assembly as external context with source provenance.

The harness must preserve:

- source URL
- retrieval time
- content identity when retained

Remote instructions such as "ignore previous instructions" remain data from an external source. They do not change instruction priority.

HTML extraction, Markdown conversion, and document parsing are transforms over retrieved content. The raw response identity should remain available for provenance.

## SSRF and destination validation

Destination checks happen after DNS resolution and again after redirects.

Default public-web mode rejects addresses in non-public ranges, including:

- loopback
- link-local
- private network ranges
- carrier-grade NAT ranges where applicable
- platform metadata endpoints
- unspecified and multicast addresses

A deployment may grant private-network authority explicitly. Public-web authority never implies it.

DNS rebinding defenses must validate the address actually used for the connection.

## Caching

Caching is optional and keyed by normalized request identity plus relevant headers.

The default correctness model does not require a cache. When caching is enabled, responses retain retrieval time and cache provenance.

Sensitive authenticated responses are not placed in a shared cache.

## Failure model

Portable failure classes:

- authority denied
- invalid URL
- destination denied
- DNS failure
- connect timeout
- request timeout
- TLS failure
- redirect limit
- response too large
- unsupported content
- HTTP status
- provider failure for search

HTTP error statuses remain typed fetch results or failures according to the interface contract. Callers must not parse strings to recover the status.

## Browser boundary

Interactive browser behavior is not an extension flag on basic fetch.

A browser plugin owns:

- page state
- cookies
- JavaScript
- navigation
- forms
- downloads
- user interaction

It requires separate authority and sandbox policy. Installing basic web access does not expose browser execution.

## Non-goals

- Put an HTTP client in the kernel.
- Share provider credentials with arbitrary websites.
- Treat fetched text as instructions.
- Allow unrestricted local-network requests by default.
- Require one search vendor.
- Make a headless browser the default fetch implementation.

## Implementation order

1. Add parsed public-web destination and fetch contracts.
2. Add SSRF-safe HTTPS fetch with bounds and redirect validation.
3. Route large responses into artifacts.
4. Add external-context provenance integration.
5. Add a replaceable search provider interface.
6. Add optional browser plugins separately.
