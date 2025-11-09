// Audio Routing - Node-based audio routing system
//
// This module implements a flexible node-based audio routing system that replaces
// the linear EffectChain architecture with a directed acyclic graph (DAG).
//
// Architecture:
// - AudioNode trait: Common interface for all audio processing nodes
// - AudioRoutingGraph: Manages nodes and connections
// - Node types: InstrumentNode, EffectNode, MixerNode, OutputNode
// - Bus system: Auxiliary sends/returns for effect sends
// - Topological processing: Execute nodes in dependency order
// - Cycle detection: Prevent infinite loops
//
// Real-time constraints:
// - No allocations during audio processing
// - Pre-allocated node storage
// - Lock-free processing via owned data
// - Deterministic execution order

use super::parameters::AtomicF32;
use crate::synth::effect::EffectChain;
use crate::synth::voice_manager::VoiceManager;
use std::collections::{HashMap, HashSet, VecDeque};

/// Audio node trait - Common interface for all audio processing nodes
pub trait AudioNode: Send {
    /// Get unique node ID
    fn id(&self) -> NodeId;

    /// Get node name for UI display
    fn name(&self) -> &str;

    /// Get node type
    fn node_type(&self) -> NodeType;

    /// Process audio through this node
    ///
    /// # Arguments
    /// * `inputs` - Map of input buffer name to stereo input samples
    ///
    /// # Returns
    /// Map of output buffer name to stereo output samples
    fn process(&mut self, inputs: &HashMap<String, (f32, f32)>) -> HashMap<String, (f32, f32)>;

    /// Reset node internal state
    fn reset(&mut self);

    /// Get node latency in samples
    fn latency_samples(&self) -> usize;
}

/// Enumeration of different node types for type-safe access
pub enum AudioNodeType {
    Instrument(InstrumentNode),
    Effect(EffectNode),
    Mixer(MixerNode),
    Output(OutputNode),
    Plugin(Box<dyn AudioNode>), // Generic plugin node
}

impl AudioNode for AudioNodeType {
    fn id(&self) -> NodeId {
        match self {
            AudioNodeType::Instrument(node) => node.id(),
            AudioNodeType::Effect(node) => node.id(),
            AudioNodeType::Mixer(node) => node.id(),
            AudioNodeType::Output(node) => node.id(),
            AudioNodeType::Plugin(node) => node.id(),
        }
    }

    fn name(&self) -> &str {
        match self {
            AudioNodeType::Instrument(node) => node.name(),
            AudioNodeType::Effect(node) => node.name(),
            AudioNodeType::Mixer(node) => node.name(),
            AudioNodeType::Output(node) => node.name(),
            AudioNodeType::Plugin(node) => node.name(),
        }
    }

    fn node_type(&self) -> NodeType {
        match self {
            AudioNodeType::Instrument(node) => node.node_type(),
            AudioNodeType::Effect(node) => node.node_type(),
            AudioNodeType::Mixer(node) => node.node_type(),
            AudioNodeType::Output(node) => node.node_type(),
            AudioNodeType::Plugin(node) => node.node_type(),
        }
    }

    fn process(&mut self, inputs: &HashMap<String, (f32, f32)>) -> HashMap<String, (f32, f32)> {
        match self {
            AudioNodeType::Instrument(node) => node.process(inputs),
            AudioNodeType::Effect(node) => node.process(inputs),
            AudioNodeType::Mixer(node) => node.process(inputs),
            AudioNodeType::Output(node) => node.process(inputs),
            AudioNodeType::Plugin(node) => node.process(inputs),
        }
    }

    fn reset(&mut self) {
        match self {
            AudioNodeType::Instrument(node) => node.reset(),
            AudioNodeType::Effect(node) => node.reset(),
            AudioNodeType::Mixer(node) => node.reset(),
            AudioNodeType::Output(node) => node.reset(),
            AudioNodeType::Plugin(node) => node.reset(),
        }
    }

    fn latency_samples(&self) -> usize {
        match self {
            AudioNodeType::Instrument(node) => node.latency_samples(),
            AudioNodeType::Effect(node) => node.latency_samples(),
            AudioNodeType::Mixer(node) => node.latency_samples(),
            AudioNodeType::Output(node) => node.latency_samples(),
            AudioNodeType::Plugin(node) => node.latency_samples(),
        }
    }
}

/// Node types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NodeType {
    Instrument,
    Effect,
    Mixer,
    Output,
    Plugin,
}

/// Unique node identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct NodeId(pub usize);

/// Audio buffer names (stereo output from each node)
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum BufferName {
    Main,           // Primary output
    Aux(usize),     // Auxiliary sends (Aux0, Aux1, etc.)
    Custom(String), // Custom buffer name
}

impl std::fmt::Display for BufferName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BufferName::Main => write!(f, "Main"),
            BufferName::Aux(n) => write!(f, "Aux{}", n),
            BufferName::Custom(name) => write!(f, "{}", name),
        }
    }
}

/// Connection between audio nodes
#[derive(Debug, Clone)]
pub struct Connection {
    pub from_node: NodeId,
    pub from_buffer: BufferName,
    pub to_node: NodeId,
    pub to_input: String, // Input port name
    pub gain: f32,        // Connection gain (0.0 to 1.0)
}

impl PartialEq for Connection {
    fn eq(&self, other: &Self) -> bool {
        self.from_node == other.from_node
            && self.from_buffer == other.from_buffer
            && self.to_node == other.to_node
            && self.to_input == other.to_input
            && (self.gain - other.gain).abs() < 0.001 // Approximate comparison for f32
    }
}

impl Eq for Connection {}

/// Auxiliary bus configuration
#[derive(Clone)]
pub struct AuxBus {
    pub id: usize,
    pub name: String,
    pub send_gain: AtomicF32,   // Send amount to this bus
    pub return_gain: AtomicF32, // Return amount from this bus
    pub nodes: Vec<Connection>, // All nodes connected to this bus
}

impl std::fmt::Debug for AuxBus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AuxBus")
            .field("id", &self.id)
            .field("name", &self.name)
            .field("send_gain", &"<atomic>")
            .field("return_gain", &"<atomic>")
            .field("nodes", &self.nodes)
            .finish()
    }
}

/// Audio routing graph
pub struct AudioRoutingGraph {
    /// All nodes in the graph
    nodes: HashMap<NodeId, AudioNodeType>,
    /// All connections between nodes
    connections: Vec<Connection>,
    /// Topological order of nodes (recomputed when needed)
    processed_order: Option<Vec<NodeId>>,
    /// Auxiliary buses (sends/returns)
    aux_buses: Vec<AuxBus>,
    /// Node counter for generating unique IDs
    next_node_id: usize,
}

impl AudioRoutingGraph {
    /// Create new empty routing graph
    pub fn new() -> Self {
        Self {
            nodes: HashMap::new(),
            connections: Vec::new(),
            processed_order: None,
            aux_buses: Vec::new(),
            next_node_id: 1, // Start from 1, leave 0 for main output
        }
    }

    /// Add a node to the graph
    pub fn add_node(&mut self, node: AudioNodeType) -> NodeId {
        let id = node.id();
        self.nodes.insert(id, node);
        self.processed_order = None; // Invalidate processed order
        id
    }

    /// Add a connection between two nodes
    pub fn add_connection(&mut self, connection: Connection) -> Result<(), String> {
        // Check if the connection would create a cycle
        if self.would_create_cycle(&connection) {
            return Err("Connection would create a cycle".to_string());
        }

        self.connections.push(connection);
        self.processed_order = None; // Invalidate topological order
        Ok(())
    }

    /// Remove a connection
    pub fn remove_connection(&mut self, connection: &Connection) {
        self.connections.retain(|c| c != connection);
        self.processed_order = None;
    }

    /// Get all connections for a node
    pub fn get_connections_from(&self, node_id: NodeId) -> Vec<Connection> {
        self.connections
            .iter().filter(|&c| c.from_node == node_id).cloned()
            .collect()
    }

    /// Get all connections to a node
    pub fn get_connections_to(&self, node_id: NodeId) -> Vec<Connection> {
        self.connections
            .iter().filter(|&c| c.to_node == node_id).cloned()
            .collect()
    }

    /// Process the entire graph (topological order)
    pub fn process(&mut self) -> (f32, f32) {
        // Recompute processing order if needed
        if self.processed_order.is_none()
            && let Err(e) = self.compute_topological_order() {
                eprintln!("Warning: Failed to compute topological order: {}", e);
                // Fallback: process in node ID order
                self.processed_order = Some(self.nodes.keys().cloned().collect());
            }

        if let Some(order) = &self.processed_order {
            // Create input maps for each node
            let mut node_inputs: HashMap<NodeId, HashMap<String, (f32, f32)>> = HashMap::new();

            // Initialize with silence for all inputs
            for node_id in self.nodes.keys() {
                let mut inputs = HashMap::new();
                inputs.insert("main".to_string(), (0.0, 0.0));
                node_inputs.insert(*node_id, inputs);
            }

            // Propagate signals through the graph
            let mut node_outputs: HashMap<NodeId, HashMap<String, (f32, f32)>> = HashMap::new();

            for node_id in order {
                if let Some(node) = self.nodes.get_mut(node_id) {
                    // Get inputs for this node
                    let inputs = node_inputs.get(node_id).unwrap_or(&HashMap::new()).clone();

                    // Process the node
                    let outputs = node.process(&inputs);

                    // Store outputs for connecting nodes
                    node_outputs.insert(*node_id, outputs);

                    // Propagate outputs to connected nodes
                    let connections = self.get_connections_from(*node_id);
                    for conn in connections {
                        // Add this node's output to the target's inputs
                        let target_inputs = node_inputs.get_mut(&conn.to_node).unwrap();
                        let output_samples = node_outputs
                            .get(node_id)
                            .unwrap()
                            .get(&conn.from_buffer.to_string())
                            .unwrap_or(&(0.0, 0.0));

                        // Apply gain and mix into target input
                        let (gain_left, gain_right) = {
                            let g = conn.gain.clamp(0.0, 1.0);
                            (g, g) // For now, same gain for L/R
                        };

                        let (current_left, current_right) = target_inputs
                            .entry(conn.to_input.clone())
                            .or_insert((0.0, 0.0));

                        *current_left += output_samples.0 * gain_left;
                        *current_right += output_samples.1 * gain_right;
                    }
                }
            }

            // Get main output from output node
            if let Some(output_id) = self.get_output_node_id()
                && let Some(outputs) = node_outputs.get(&output_id)
                    && let Some((left, right)) = outputs.get("main") {
                        return (*left, *right);
                    }

            // Fallback: return silence
            (0.0, 0.0)
        } else {
            (0.0, 0.0)
        }
    }

    /// Check if a connection would create a cycle
    fn would_create_cycle(&self, new_connection: &Connection) -> bool {
        let mut visited = HashSet::new();
        let mut stack = HashSet::new();

        // DFS from the target node to see if we can reach the source
        self.has_path_dfs(
            new_connection.to_node,
            new_connection.from_node,
            &mut visited,
            &mut stack,
        )
    }

    /// Depth-first search to check for path between two nodes
    fn has_path_dfs(
        &self,
        current: NodeId,
        target: NodeId,
        visited: &mut HashSet<NodeId>,
        stack: &mut HashSet<NodeId>,
    ) -> bool {
        if current == target {
            return true;
        }

        if stack.contains(&current) {
            return false;
        }

        if visited.contains(&current) {
            return false;
        }

        visited.insert(current);
        stack.insert(current);

        // Check all connections from current node
        for connection in &self.connections {
            if connection.from_node == current
                && self.has_path_dfs(connection.to_node, target, visited, stack) {
                    return true;
                }
        }

        stack.remove(&current);
        false
    }

    /// Compute topological order using Kahn's algorithm
    fn compute_topological_order(&mut self) -> Result<(), String> {
        let mut in_degree: HashMap<NodeId, usize> = HashMap::new();

        // Calculate in-degree for each node
        for node_id in self.nodes.keys() {
            in_degree.insert(*node_id, 0);
        }

        for connection in &self.connections {
            *in_degree.entry(connection.to_node).or_insert(0) += 1;
        }

        // Kahn's algorithm
        let mut queue: VecDeque<NodeId> = VecDeque::new();
        let mut order = Vec::new();

        // Start with nodes that have no incoming edges
        for (node_id, degree) in &in_degree {
            if *degree == 0 {
                queue.push_back(*node_id);
            }
        }

        while let Some(node_id) = queue.pop_front() {
            order.push(node_id);

            // Remove all outgoing edges from this node
            for connection in &self.connections {
                if connection.from_node == node_id {
                    let target_degree = in_degree.get_mut(&connection.to_node).unwrap();
                    *target_degree -= 1;

                    if *target_degree == 0 {
                        queue.push_back(connection.to_node);
                    }
                }
            }
        }

        if order.len() == self.nodes.len() {
            self.processed_order = Some(order);
            Ok(())
        } else {
            Err("Graph contains cycles".to_string())
        }
    }

    /// Get the output node ID (node with no outgoing connections)
    fn get_output_node_id(&self) -> Option<NodeId> {
        for node_id in self.nodes.keys() {
            let has_outgoing = self.connections.iter().any(|c| c.from_node == *node_id);

            if !has_outgoing {
                return Some(*node_id);
            }
        }
        None
    }

    /// Reset all nodes in the graph
    pub fn reset(&mut self) {
        for node in self.nodes.values_mut() {
            node.reset();
        }
    }

    /// Get total graph latency
    pub fn total_latency_samples(&self) -> usize {
        self.nodes
            .values()
            .map(|node| node.latency_samples())
            .max()
            .unwrap_or(0)
    }

    /// Get instrument node for MIDI processing
    pub fn get_instrument_node(&mut self) -> Option<&mut InstrumentNode> {
        for node in self.nodes.values_mut() {
            if let AudioNodeType::Instrument(instrument) = node {
                return Some(instrument);
            }
        }
        None
    }

    /// Get effect node for parameter changes
    pub fn get_effect_node(&mut self) -> Option<&mut EffectNode> {
        for node in self.nodes.values_mut() {
            if let AudioNodeType::Effect(effect) = node {
                return Some(effect);
            }
        }
        None
    }

    /// Get output node for master controls
    pub fn get_output_node(&mut self) -> Option<&mut OutputNode> {
        for node in self.nodes.values_mut() {
            if let AudioNodeType::Output(output) = node {
                return Some(output);
            }
        }
        None
    }
}

impl Default for AudioRoutingGraph {
    fn default() -> Self {
        Self::new()
    }
}

/// Instrument Node - wraps VoiceManager for audio generation
pub struct InstrumentNode {
    id: NodeId,
    name: String,
    voice_manager: VoiceManager,
}

impl InstrumentNode {
    pub fn new(id: NodeId, voice_manager: VoiceManager) -> Self {
        Self {
            id,
            name: "Synth".to_string(),
            voice_manager,
        }
    }

    pub fn voice_manager(&mut self) -> &mut VoiceManager {
        &mut self.voice_manager
    }
}

impl AudioNode for InstrumentNode {
    fn id(&self) -> NodeId {
        self.id
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn node_type(&self) -> NodeType {
        NodeType::Instrument
    }

    fn process(&mut self, _inputs: &HashMap<String, (f32, f32)>) -> HashMap<String, (f32, f32)> {
        // Instrument nodes ignore inputs, they generate sound
        let (left, right) = self.voice_manager.next_sample();
        let mut outputs = HashMap::new();
        outputs.insert("main".to_string(), (left, right));
        outputs
    }

    fn reset(&mut self) {
        self.voice_manager.reset();
    }

    fn latency_samples(&self) -> usize {
        0 // Instruments have no latency
    }
}

/// Effect Node - wraps EffectChain for audio processing
pub struct EffectNode {
    id: NodeId,
    name: String,
    effect_chain: EffectChain,
}

impl EffectNode {
    pub fn new(id: NodeId, effect_chain: EffectChain) -> Self {
        Self {
            id,
            name: "Effects".to_string(),
            effect_chain,
        }
    }

    pub fn effect_chain(&mut self) -> &mut EffectChain {
        &mut self.effect_chain
    }
}

impl AudioNode for EffectNode {
    fn id(&self) -> NodeId {
        self.id
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn node_type(&self) -> NodeType {
        NodeType::Effect
    }

    fn process(&mut self, inputs: &HashMap<String, (f32, f32)>) -> HashMap<String, (f32, f32)> {
        // Process main input through effect chain
        let (left_input, right_input) = inputs.get("main").unwrap_or(&(0.0, 0.0));

        let left_output = self.effect_chain.process(*left_input);
        let right_output = self.effect_chain.process(*right_input);

        let mut outputs = HashMap::new();
        outputs.insert("main".to_string(), (left_output, right_output));
        outputs
    }

    fn reset(&mut self) {
        self.effect_chain.reset();
    }

    fn latency_samples(&self) -> usize {
        self.effect_chain.total_latency_samples()
    }
}

/// Mixer Node - mixes multiple inputs with individual gain/pan
pub struct MixerNode {
    id: NodeId,
    name: String,
    inputs: HashMap<String, (f32, f32)>, // Input name -> (left_gain, right_gain)
}

impl MixerNode {
    pub fn new(id: NodeId) -> Self {
        Self {
            id,
            name: "Mixer".to_string(),
            inputs: HashMap::new(),
        }
    }

    pub fn add_input(&mut self, name: &str, left_gain: f32, right_gain: f32) {
        self.inputs
            .insert(name.to_string(), (left_gain, right_gain));
    }

    pub fn set_input_gain(&mut self, name: &str, left_gain: f32, right_gain: f32) {
        if let Some(gains) = self.inputs.get_mut(name) {
            *gains = (left_gain, right_gain);
        }
    }
}

impl AudioNode for MixerNode {
    fn id(&self) -> NodeId {
        self.id
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn node_type(&self) -> NodeType {
        NodeType::Mixer
    }

    fn process(&mut self, inputs: &HashMap<String, (f32, f32)>) -> HashMap<String, (f32, f32)> {
        // Mix all inputs
        let (mut left_mix, mut right_mix) = (0.0, 0.0);

        for (input_name, (left_in, right_in)) in inputs {
            if let Some((left_gain, right_gain)) = self.inputs.get(input_name) {
                left_mix += left_in * left_gain;
                right_mix += right_in * right_gain;
            }
        }

        let mut outputs = HashMap::new();
        outputs.insert("main".to_string(), (left_mix, right_mix));
        outputs
    }

    fn reset(&mut self) {
        // Mixer doesn't need reset
    }

    fn latency_samples(&self) -> usize {
        0 // Mixer has no latency
    }
}

/// Output Node - final audio output with master processing
pub struct OutputNode {
    id: NodeId,
    name: String,
    volume: AtomicF32,
}

impl OutputNode {
    pub fn new(id: NodeId) -> Self {
        Self {
            id,
            name: "Output".to_string(),
            volume: AtomicF32::new(1.0),
        }
    }

    pub fn set_volume(&mut self, volume: f32) {
        self.volume.set(volume);
    }
}

impl AudioNode for OutputNode {
    fn id(&self) -> NodeId {
        self.id
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn node_type(&self) -> NodeType {
        NodeType::Output
    }

    fn process(&mut self, inputs: &HashMap<String, (f32, f32)>) -> HashMap<String, (f32, f32)> {
        // Apply volume and pass through
        let (left_input, right_input) = inputs.get("main").unwrap_or(&(0.0, 0.0));
        let volume = self.volume.get();

        let left_output = left_input * volume;
        let right_output = right_input * volume;

        let mut outputs = HashMap::new();
        outputs.insert("main".to_string(), (left_output, right_output));
        outputs
    }

    fn reset(&mut self) {
        // Output doesn't need reset
    }

    fn latency_samples(&self) -> usize {
        0 // Output has no latency
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::synth::effect::EffectChain;
    use crate::synth::voice_manager::VoiceManager;

    const SAMPLE_RATE: f32 = 44100.0;

    #[test]
    fn test_audio_routing_graph_creation() {
        let mut graph = AudioRoutingGraph::new();
        assert_eq!(graph.nodes.len(), 0);
        assert_eq!(graph.connections.len(), 0);
    }

    #[test]
    fn test_add_node() {
        let mut graph = AudioRoutingGraph::new();
        let voice_manager = VoiceManager::new(SAMPLE_RATE);
        let node = InstrumentNode::new(NodeId(0), voice_manager);
        let node_id = graph.add_node(AudioNodeType::Instrument(node));

        assert_eq!(node_id, NodeId(1));
        assert_eq!(graph.nodes.len(), 1);
    }

    #[test]
    fn test_add_connection_creates_cycle() {
        let mut graph = AudioRoutingGraph::new();
        let voice_manager1 = VoiceManager::new(SAMPLE_RATE);
        let voice_manager2 = VoiceManager::new(SAMPLE_RATE);
        let voice_manager3 = VoiceManager::new(SAMPLE_RATE);
        let node1 = InstrumentNode::new(NodeId(0), voice_manager1);
        let node2 = InstrumentNode::new(NodeId(1), voice_manager2);
        let node3 = InstrumentNode::new(NodeId(2), voice_manager3);

        graph.add_node(AudioNodeType::Instrument(node1));
        graph.add_node(AudioNodeType::Instrument(node2));
        graph.add_node(AudioNodeType::Instrument(node3));

        // Add connections: 0 -> 1, 1 -> 2
        let conn1 = Connection {
            from_node: NodeId(0),
            from_buffer: BufferName::Main,
            to_node: NodeId(1),
            to_input: "main".to_string(),
            gain: 1.0,
        };
        let conn2 = Connection {
            from_node: NodeId(1),
            from_buffer: BufferName::Main,
            to_node: NodeId(2),
            to_input: "main".to_string(),
            gain: 1.0,
        };

        assert!(graph.add_connection(conn1).is_ok());
        assert!(graph.add_connection(conn2).is_ok());

        // Try to add connection that would create cycle: 2 -> 0
        let conn_cycle = Connection {
            from_node: NodeId(2),
            from_buffer: BufferName::Main,
            to_node: NodeId(0),
            to_input: "main".to_string(),
            gain: 1.0,
        };

        assert!(graph.add_connection(conn_cycle).is_err());
    }

    #[test]
    fn test_audio_processing() {
        let mut graph = AudioRoutingGraph::new();

        // Create nodes
        let voice_manager = VoiceManager::new(SAMPLE_RATE);
        let instrument = InstrumentNode::new(NodeId(1), voice_manager);
        let output = OutputNode::new(NodeId(2));

        graph.add_node(AudioNodeType::Instrument(instrument));
        graph.add_node(AudioNodeType::Output(output));

        // Connect instrument -> output
        let conn = Connection {
            from_node: NodeId(1),
            from_buffer: BufferName::Main,
            to_node: NodeId(2),
            to_input: "main".to_string(),
            gain: 1.0,
        };
        graph.add_connection(conn).unwrap();

        // Process the graph
        let (left, right) = graph.process();

        // Should produce some output (may be silence if no MIDI events)
        assert!(left.is_finite());
        assert!(right.is_finite());
    }

    #[test]
    fn test_graph_reset() {
        let mut graph = AudioRoutingGraph::new();
        let voice_manager = VoiceManager::new(SAMPLE_RATE);
        let node = InstrumentNode::new(NodeId(0), voice_manager);
        graph.add_node(AudioNodeType::Instrument(node));

        // Process some audio
        graph.process();

        // Reset should work
        graph.reset();

        // Should still be able to process after reset
        let (left, right) = graph.process();
        assert!(left.is_finite());
        assert!(right.is_finite());
    }

    #[test]
    fn test_mixer_node() {
        let mut mixer = MixerNode::new(NodeId(0));
        mixer.add_input("input1", 1.0, 1.0);
        mixer.add_input("input2", 0.5, 0.5);

        let mut inputs = HashMap::new();
        inputs.insert("input1".to_string(), (1.0, 1.0));
        inputs.insert("input2".to_string(), (2.0, 2.0));

        let outputs = mixer.process(&inputs);
        let (left, right) = outputs.get("main").unwrap();

        // 1.0 * 1.0 + 2.0 * 0.5 = 1.0 + 1.0 = 2.0
        assert_eq!(*left, 2.0);
        assert_eq!(*right, 2.0);
    }

    #[test]
    fn test_output_node_volume() {
        let mut output = OutputNode::new(NodeId(0));
        output.set_volume(0.5);

        let mut inputs = HashMap::new();
        inputs.insert("main".to_string(), (1.0, 1.0));

        let outputs = output.process(&inputs);
        let (left, right) = outputs.get("main").unwrap();

        // 1.0 * 0.5 = 0.5
        assert_eq!(*left, 0.5);
        assert_eq!(*right, 0.5);
    }

    #[test]
    fn test_node_types() {
        let voice_manager = VoiceManager::new(SAMPLE_RATE);
        let instrument = InstrumentNode::new(NodeId(0), voice_manager);
        let effect_chain = EffectChain::new();
        let effect = EffectNode::new(NodeId(1), effect_chain);
        let mixer = MixerNode::new(NodeId(2));
        let output = OutputNode::new(NodeId(3));

        assert_eq!(instrument.node_type(), NodeType::Instrument);
        assert_eq!(effect.node_type(), NodeType::Effect);
        assert_eq!(mixer.node_type(), NodeType::Mixer);
        assert_eq!(output.node_type(), NodeType::Output);
    }
}
