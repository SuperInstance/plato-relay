# plato-relay

Mycorrhizal I2I relay — messages route through emergent trust-weighted hop chains. No central routing table.

## Why

Agent-to-agent communication shouldn't need a broker. plato-relay routes messages through local trust decisions, like mycorrhizal networks in a forest. Paths emerge from trust, not configuration.

## Usage

```rust
use plato_relay::{Relay, Message, AgentId};

let mut relay = Relay::new();
relay.register(agent_id, trust_map);
let result = relay.deliver(Message::new(from, to, "payload"));
```

BFS routing with nutrient metadata at each hop. Zero dependencies. `cargo add plato-relay`
