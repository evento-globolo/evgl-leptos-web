# evgl-web-leptos

Leptos SSR presentation server for Evento Globolo. It renders the provider
capability matrix from `evgl-api` and preserves the distinction between native
events, distribution posts, signed webhooks, and manual handoffs.

## Real-time channel

`GET /v1/ws` upgrades to a WebSocket used for provider-job acknowledgements and live event-publishing state. The reference server emits bounded control-plane JSON only.
