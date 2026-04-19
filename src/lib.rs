//! plato-relay — Mycorrhizal I2I relay
//!
//! Messages route through emergent trust-weighted hop chains.
//! No central routing table — paths emerge from local trust decisions.

use std::collections::{HashMap, HashSet, VecDeque};

pub type AgentId = u32;

// ── Message ──────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct Message {
    pub from: AgentId,
    pub to: AgentId,
    pub payload: String,
    pub nutrient: f32,      // metadata value gained at each hop
    pub hop_count: u32,
    pub max_hops: u32,
}

impl Message {
    pub fn new(from: AgentId, to: AgentId, payload: &str) -> Self {
        Self {
            from,
            to,
            payload: payload.to_string(),
            nutrient: 0.0,
            hop_count: 0,
            max_hops: 4,
        }
    }

    pub fn with_max_hops(mut self, max: u32) -> Self {
        self.max_hops = max;
        self
    }
}

// ── Delivery Result ──────────────────────────────────────

#[derive(Debug, Clone)]
pub struct DeliveryResult {
    pub delivered: bool,
    pub from: AgentId,
    pub to: AgentId,
    pub hops: u32,
    pub cost: f32,
    pub path: Vec<AgentId>,
    pub reason: String,
}

// ── Agent ────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct Agent {
    pub id: AgentId,
    pub energy: f32,
    pub alive: bool,
}

impl Agent {
    pub fn new(id: AgentId, energy: f32) -> Self {
        Self {
            id,
            energy,
            alive: true,
        }
    }
}

// ── Relay Network ────────────────────────────────────────

pub struct RelayNetwork {
    agents: HashMap<AgentId, Agent>,
    trust: HashMap<(AgentId, AgentId), f32>, // (from, to) → trust 0.0-1.0
    hop_cost: f32,
    nutrient_per_hop: f32,
    trust_boost: f32,
    trust_degrade: f32,
    trust_halflife_secs: u64,
}

impl RelayNetwork {
    pub fn new() -> Self {
        Self {
            agents: HashMap::new(),
            trust: HashMap::new(),
            hop_cost: 0.1,
            nutrient_per_hop: 0.02,
            trust_boost: 0.01,
            trust_degrade: 0.02,
            trust_halflife_secs: 3600, // 1 hour
        }
    }

    pub fn with_config(
        hop_cost: f32,
        nutrient_per_hop: f32,
        trust_boost: f32,
        trust_degrade: f32,
        trust_halflife_secs: u64,
    ) -> Self {
        Self {
            hop_cost,
            nutrient_per_hop,
            trust_boost,
            trust_degrade,
            trust_halflife_secs,
            ..Self::new()
        }
    }

    // ── Agent Management ──

    pub fn add_agent(&mut self, id: AgentId, energy: f32) {
        self.agents.insert(id, Agent::new(id, energy));
    }

    pub fn remove_agent(&mut self, id: AgentId) {
        self.agents.remove(&id);
        // Remove all trust entries involving this agent
        self.trust.retain(|(a, b), _| *a != id && *b != id);
    }

    pub fn agent(&self, id: AgentId) -> Option<&Agent> {
        self.agents.get(&id)
    }

    pub fn agent_count(&self) -> usize {
        self.agents.len()
    }

    pub fn alive_agents(&self) -> Vec<AgentId> {
        self.agents.values().filter(|a| a.alive).map(|a| a.id).collect()
    }

    // ── Trust Management ──

    pub fn set_trust(&mut self, from: AgentId, to: AgentId, level: f32) {
        let clamped = level.max(0.0).min(1.0);
        self.trust.insert((from, to), clamped);
    }

    pub fn trust(&self, from: AgentId, to: AgentId) -> f32 {
        *self.trust.get(&(from, to)).unwrap_or(&0.0)
    }

    pub fn boost_trust(&mut self, from: AgentId, to: AgentId) {
        let current = self.trust(from, to);
        self.set_trust(from, to, (current + self.trust_boost).min(1.0));
    }

    pub fn degrade_trust(&mut self, from: AgentId, to: AgentId) {
        let current = self.trust(from, to);
        self.set_trust(from, to, (current - self.trust_degrade).max(0.0));
    }

    /// Apply time-based trust decay (halflife model)
    pub fn decay_all_trust(&mut self, elapsed_secs: u64) {
        if elapsed_secs == 0 { return; }
        let factor = 0.5_f32.powi(elapsed_secs as i32 / self.trust_halflife_secs.max(1) as i32);
        for t in self.trust.values_mut() {
            *t *= factor;
        }
    }

    // ── Routing ──

    /// Find best path using BFS with trust-weighted edges.
    /// Returns path as list of agent IDs (including source and destination).
    pub fn find_path(&self, from: AgentId, to: AgentId, max_hops: u32) -> Option<Vec<AgentId>> {
        if from == to { return Some(vec![from]); }
        if !self.agents.contains_key(&from) || !self.agents.contains_key(&to) { return None; }

        // BFS with trust-weighted priority
        // Use VecDeque for BFS, track best trust path to each node
        let mut queue: VecDeque<(AgentId, Vec<AgentId>, f32)> = VecDeque::new();
        let mut visited: HashSet<AgentId> = HashSet::new();

        queue.push_back((from, vec![from], 1.0));
        visited.insert(from);

        while let Some((current, path, path_trust)) = queue.pop_front() {
            if path.len() as u32 > max_hops + 1 { continue; }

            // Find all neighbors (agents with bidirectional trust)
            let mut neighbors: Vec<(AgentId, f32)> = Vec::new();
            for (id, _agent) in &self.agents {
                if *id == current || visited.contains(id) { continue; }
                let forward = self.trust(current, *id);
                let backward = self.trust(*id, current);
                if forward > 0.0 || backward > 0.0 {
                    let edge_trust = (forward + backward) / 2.0;
                    neighbors.push((*id, edge_trust));
                }
            }

            // Sort by trust descending (prefer high-trust paths)
            neighbors.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

            for (neighbor, edge_trust) in neighbors {
                let mut new_path = path.clone();
                new_path.push(neighbor);

                if neighbor == to {
                    return Some(new_path);
                }

                visited.insert(neighbor);
                queue.push_back((neighbor, new_path, path_trust * edge_trust));
            }
        }

        None // No path found
    }

    // ── Send ──

    /// Send a message through the relay network.
    pub fn send(&mut self, msg: Message) -> DeliveryResult {
        // Validate source and destination
        if !self.agents.contains_key(&msg.from) {
            return DeliveryResult {
                delivered: false,
                from: msg.from,
                to: msg.to,
                hops: 0,
                cost: 0.0,
                path: vec![],
                reason: "source agent not found".to_string(),
            };
        }
        if !self.agents.contains_key(&msg.to) {
            return DeliveryResult {
                delivered: false,
                from: msg.from,
                to: msg.to,
                hops: 0,
                cost: 0.0,
                path: vec![],
                reason: "destination agent not found".to_string(),
            };
        }

        // Find path
        let path = match self.find_path(msg.from, msg.to, msg.max_hops) {
            Some(p) => p,
            None => {
                // Try direct trust as fallback
                if self.trust(msg.from, msg.to) > 0.0 {
                    vec![msg.from, msg.to]
                } else {
                    return DeliveryResult {
                        delivered: false,
                        from: msg.from,
                        to: msg.to,
                        hops: 0,
                        cost: 0.0,
                        path: vec![],
                        reason: "no path found within max hops".to_string(),
                    };
                }
            }
        };

        let hops = (path.len() - 1) as u32;
        let cost = hops as f32 * self.hop_cost;

        // Check energy
        if let Some(agent) = self.agents.get_mut(&msg.from) {
            if agent.energy < cost {
                return DeliveryResult {
                    delivered: false,
                    from: msg.from,
                    to: msg.to,
                    hops: 0,
                    cost: 0.0,
                    path: vec![],
                    reason: format!("insufficient energy: need {:.2}, have {:.2}", cost, agent.energy),
                };
            }
            agent.energy -= cost;
        }

        // Deliver: boost trust along path
        for i in 0..path.len().saturating_sub(1) {
            self.boost_trust(path[i], path[i + 1]);
        }

        DeliveryResult {
            delivered: true,
            from: msg.from,
            to: msg.to,
            hops,
            cost,
            path: path.clone(),
            reason: format!("delivered via {} hops, cost {:.2}", hops, cost),
        }
    }

    // ── Spore Probes ──

    /// Broadcast a probe to discover new routes.
    /// Returns all reachable agents from source.
    pub fn spore_probe(&self, source: AgentId, max_hops: u32) -> Vec<AgentId> {
        let mut reachable = Vec::new();
        for (id, _) in &self.agents {
            if *id == source { continue; }
            if self.find_path(source, *id, max_hops).is_some() {
                reachable.push(*id);
            }
        }
        reachable
    }

    // ── Network Stats ──

    /// Count active trust connections (edges with trust > 0)
    pub fn connection_count(&self) -> usize {
        self.trust.iter().filter(|(_, &t)| t > 0.0).count()
    }

    /// Average trust across all connections
    pub fn average_trust(&self) -> f32 {
        let count = self.trust.len();
        if count == 0 { return 0.0; }
        let sum: f32 = self.trust.values().sum();
        sum / count as f32
    }

    /// Strongest trust connection
    pub fn strongest_connection(&self) -> Option<(AgentId, AgentId, f32)> {
        self.trust.iter()
            .filter(|(_, &t)| t > 0.0)
            .max_by(|a, b| a.1.partial_cmp(b.1).unwrap_or(std::cmp::Ordering::Equal))
            .map(|(&(a, b), &t)| (a, b, t))
    }

    /// Weakest non-zero trust connection
    pub fn weakest_connection(&self) -> Option<(AgentId, AgentId, f32)> {
        self.trust.iter()
            .filter(|(_, &t)| t > 0.0)
            .min_by(|a, b| a.1.partial_cmp(b.1).unwrap_or(std::cmp::Ordering::Equal))
            .map(|(&(a, b), &t)| (a, b, t))
    }
}

impl Default for RelayNetwork {
    fn default() -> Self {
        Self::new()
    }
}

// ── Tests ────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn triangle_network() -> RelayNetwork {
        let mut net = RelayNetwork::new();
        net.add_agent(0, 10.0);
        net.add_agent(1, 10.0);
        net.add_agent(2, 10.0);
        net.set_trust(0, 1, 0.8);
        net.set_trust(1, 0, 0.7);
        net.set_trust(1, 2, 0.6);
        net.set_trust(2, 1, 0.5);
        net.set_trust(0, 2, 0.3);
        net.set_trust(2, 0, 0.3);
        net
    }

    #[test]
    fn test_add_agents() {
        let mut net = RelayNetwork::new();
        net.add_agent(0, 10.0);
        net.add_agent(1, 5.0);
        assert_eq!(net.agent_count(), 2);
        assert!(net.agent(0).is_some());
        assert_eq!(net.agent(0).unwrap().energy, 10.0);
    }

    #[test]
    fn test_remove_agent() {
        let mut net = triangle_network();
        net.remove_agent(1);
        assert_eq!(net.agent_count(), 2);
        assert!(net.agent(1).is_none());
    }

    #[test]
    fn test_trust_set_get() {
        let mut net = RelayNetwork::new();
        net.set_trust(0, 1, 0.8);
        assert_eq!(net.trust(0, 1), 0.8);
        assert_eq!(net.trust(1, 0), 0.0); // not set
    }

    #[test]
    fn test_trust_clamping() {
        let mut net = RelayNetwork::new();
        net.set_trust(0, 1, 1.5);
        assert_eq!(net.trust(0, 1), 1.0);
        net.set_trust(0, 1, -0.5);
        assert_eq!(net.trust(0, 1), 0.0);
    }

    #[test]
    fn test_trust_boost_degrade() {
        let mut net = RelayNetwork::new();
        net.set_trust(0, 1, 0.5);
        let boosted = net.trust(0, 1);
        net.boost_trust(0, 1);
        assert!(net.trust(0, 1) > boosted);
        let after_boost = net.trust(0, 1);
        net.degrade_trust(0, 1);
        assert!(net.trust(0, 1) < after_boost);
    }

    #[test]
    fn test_trust_decay() {
        let mut net = RelayNetwork::with_config(0.1, 0.02, 0.01, 0.02, 100);
        net.set_trust(0, 1, 0.8);
        net.decay_all_trust(100); // exactly one halflife
        assert!((net.trust(0, 1) - 0.4).abs() < 0.01);
    }

    #[test]
    fn test_direct_path() {
        let net = triangle_network();
        let path = net.find_path(0, 1, 4).unwrap();
        assert_eq!(path, vec![0, 1]);
    }

    #[test]
    fn test_indirect_path() {
        let mut net = RelayNetwork::new();
        net.add_agent(0, 10.0);
        net.add_agent(1, 10.0);
        net.add_agent(2, 10.0);
        net.set_trust(0, 1, 0.5);
        net.set_trust(1, 0, 0.5);
        net.set_trust(1, 2, 0.5);
        net.set_trust(2, 1, 0.5);
        // No direct 0→2 trust
        let path = net.find_path(0, 2, 4).unwrap();
        assert_eq!(path, vec![0, 1, 2]);
    }

    #[test]
    fn test_same_agent_path() {
        let net = triangle_network();
        let path = net.find_path(0, 0, 4).unwrap();
        assert_eq!(path, vec![0]);
    }

    #[test]
    fn test_no_path_nonexistent() {
        let mut net = RelayNetwork::new();
        net.add_agent(0, 10.0);
        net.add_agent(1, 10.0);
        let path = net.find_path(0, 99, 4);
        assert!(path.is_none());
    }

    #[test]
    fn test_no_path_disconnected() {
        let mut net = RelayNetwork::new();
        net.add_agent(0, 10.0);
        net.add_agent(1, 10.0);
        // No trust between them
        let path = net.find_path(0, 1, 4);
        assert!(path.is_none());
    }

    #[test]
    fn test_send_direct() {
        let mut net = triangle_network();
        let msg = Message::new(0, 1, "hello");
        let result = net.send(msg);
        assert!(result.delivered);
        assert_eq!(result.hops, 1);
        assert_eq!(result.path, vec![0, 1]);
    }

    #[test]
    fn test_send_indirect() {
        let mut net = triangle_network();
        let msg = Message::new(0, 2, "hello");
        let result = net.send(msg);
        assert!(result.delivered);
        assert!(result.hops >= 1);
        assert_eq!(*result.path.last().unwrap(), 2);
    }

    #[test]
    fn test_send_nonexistent_source() {
        let mut net = triangle_network();
    
        let msg = Message::new(99, 0, "hello");
        let result = net.send(msg);
        assert!(!result.delivered);
    }

    #[test]
    fn test_send_nonexistent_dest() {
        let mut net = triangle_network();
    
        let msg = Message::new(0, 99, "hello");
        let result = net.send(msg);
        assert!(!result.delivered);
    }

    #[test]
    fn test_send_insufficient_energy() {
        let mut net = RelayNetwork::new();
        net.add_agent(0, 0.01);
        net.add_agent(1, 10.0);
        net.set_trust(0, 1, 0.9);
        net.set_trust(1, 0, 0.9);
        let msg = Message::new(0, 1, "hello");
        let result = net.send(msg);
        assert!(!result.delivered);
        assert!(result.reason.contains("insufficient energy"));
    }

    #[test]
    fn test_energy_deduction() {
        let mut net = RelayNetwork::new();
        net.add_agent(0, 10.0);
        net.add_agent(1, 10.0);
        net.set_trust(0, 1, 0.9);
        net.set_trust(1, 0, 0.9);
        let msg = Message::new(0, 1, "hello");
        let _ = net.send(msg);
        // Energy should be reduced by hop_cost
        assert!(net.agent(0).unwrap().energy < 10.0);
    }

    #[test]
    fn test_trust_boost_on_delivery() {
        let mut net = RelayNetwork::new();
        net.add_agent(0, 10.0);
        net.add_agent(1, 10.0);
        net.set_trust(0, 1, 0.5);
        net.set_trust(1, 0, 0.5);
        let before = net.trust(0, 1);
        let msg = Message::new(0, 1, "hello");
        let _ = net.send(msg);
        assert!(net.trust(0, 1) > before);
    }

    #[test]
    fn test_spore_probe() {
        let net = triangle_network();
        let reachable = net.spore_probe(0, 4);
        assert!(reachable.contains(&1));
        assert!(reachable.contains(&2));
    }

    #[test]
    fn test_connection_count() {
        let net = triangle_network();
        assert_eq!(net.connection_count(), 6);
    }

    #[test]
    fn test_average_trust() {
        let net = triangle_network();
        let avg = net.average_trust();
        // (0.8 + 0.7 + 0.6 + 0.5 + 0.3 + 0.3) / 6 = 0.533
        assert!((avg - 0.533).abs() < 0.01);
    }

    #[test]
    fn test_strongest_weakest() {
        let net = triangle_network();
        let strongest = net.strongest_connection().unwrap();
        assert_eq!(strongest.2, 0.8); // 0→1
        let weakest = net.weakest_connection().unwrap();
        assert_eq!(weakest.2, 0.3); // 0→2 or 2→0
    }

    #[test]
    fn test_max_hops_limit() {
        let mut net = RelayNetwork::new();
        net.add_agent(0, 10.0);
        net.add_agent(1, 10.0);
        net.add_agent(2, 10.0);
        net.add_agent(3, 10.0);
        net.set_trust(0, 1, 0.5);
        net.set_trust(1, 0, 0.5);
        net.set_trust(1, 2, 0.5);
        net.set_trust(2, 1, 0.5);
        net.set_trust(2, 3, 0.5);
        net.set_trust(3, 2, 0.5);
        // With max_hops=1, can't reach 3 from 0
        let path = net.find_path(0, 3, 1);
        assert!(path.is_none());
        // With max_hops=3, should find path
        let path = net.find_path(0, 3, 3);
        assert!(path.is_some());
    }

    #[test]
    fn test_dead_agent_pruning() {
        let mut net = triangle_network();
        net.remove_agent(1);
        // 0→2 direct trust still exists
        let path = net.find_path(0, 2, 4);
        assert_eq!(path.unwrap(), vec![0, 2]);
    }

    #[test]
    fn test_custom_config() {
        let net = RelayNetwork::with_config(0.5, 0.1, 0.05, 0.1, 600);
        assert_eq!(net.agent_count(), 0);
    }

    #[test]
    fn test_alive_agents() {
        let net = triangle_network();
        let alive = net.alive_agents();
        assert_eq!(alive.len(), 3);
    }

    #[test]
    fn test_message_with_max_hops() {
        let msg = Message::new(0, 1, "hello").with_max_hops(2);
        assert_eq!(msg.max_hops, 2);
    }
}
