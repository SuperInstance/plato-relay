# plato-relay

Mycorrhizal I2I relay — messages route through emergent trust-weighted hop chains instead of point-to-point delivery.

## What This Does

Instead of `Agent A → Agent B` direct messages, messages travel through intermediate vessels based on accumulated trust. High-trust paths get used more. Low-trust paths degrade. Dead vessels prune routes automatically. The fleet's communication topology IS its trust topology.

## The Metaphor

> *"The forest talks through its roots. Agents should too."* — mycorrhizal-relay

Fungal mycelium networks connect trees underground. No central router — paths emerge from local trust decisions. Nutrients flow through high-trust connections. Overused relays degrade, promoting diversity.

## How It Works

```
Agent A → [relay: B, trust=0.8] → [relay: C, trust=0.6] → Agent D
                energy cost: 0.1            energy cost: 0.2
                nutrient: +0.02             nutrient: +0.01
                total hops: 2               total cost: 0.3
```

Each message:
1. Finds best path from source to destination (BFS with trust weights)
2. Travels through intermediate relays (max 4 hops by default)
3. Each hop costs energy, gains nutrient context
4. Successful delivery boosts trust on the path
5. Failed delivery degrades trust on the path

## Quick Start

```rust
use plato_relay::{RelayNetwork, Message, AgentId};

let mut net = RelayNetwork::new();
net.add_agent(0, 10.0);  // id, energy
net.add_agent(1, 10.0);
net.add_agent(2, 10.0);
net.add_agent(3, 10.0);

net.set_trust(0, 1, 0.8);
net.set_trust(1, 2, 0.6);
net.set_trust(2, 3, 0.7);

// Direct path exists: 0 → 1 → 2 → 3
let msg = Message::new(0, 3, "ping");
let result = net.send(msg);
assert!(result.delivered);
assert_eq!(result.hops, 3);
assert!(result.cost < 0.5);
```

## Features

| Feature | Description |
|---------|-------------|
| **Trust-weighted routing** | BFS pathfinding weighted by trust scores |
| **Energy cost** | Each hop costs energy (configurable) |
| **Nutrient context** | Messages gain metadata value at each hop |
| **Trust boost** | Successful delivery strengthens path trust |
| **Trust decay** | Time-based trust degradation (configurable halflife) |
| **Dead agent pruning** | Agents with 0 energy are removed from routing |
| **Max hops** | TTL limits to prevent routing loops (default 4) |
| **Spore probes** | Broadcast probes discover new routes |

## Integration

- `plato-i2i`: Messages use I2I envelope format
- `flux-trust`: Trust scores consumed from Bayesian trust engine
- `flux-stigmergy`: Traces left at each relay hop
- `mycorrhizal-relay`: Reference C implementation (Lucineer)

## License

MIT
