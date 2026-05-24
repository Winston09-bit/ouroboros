use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet, VecDeque};
use uuid::Uuid;

/// Nodetyp – ekonomisk entitet
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum NodeType {
    Company,      // LandveX AB
    Merchant,     // ICA
    Supplier,     // Företag som fakturerat
    BankAccount,  // Konto
    Transaction,  // Banktransaktion
    Invoice,      // Faktura
    Receipt,      // Kvitto
    Voucher,      // ERP-verifikation
    Person,       // Anställd, motpart
    TaxAuthority, // Skatteverket etc
}

impl std::fmt::Display for NodeType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            NodeType::Company => "Company",
            NodeType::Merchant => "Merchant",
            NodeType::Supplier => "Supplier",
            NodeType::BankAccount => "BankAccount",
            NodeType::Transaction => "Transaction",
            NodeType::Invoice => "Invoice",
            NodeType::Receipt => "Receipt",
            NodeType::Voucher => "Voucher",
            NodeType::Person => "Person",
            NodeType::TaxAuthority => "TaxAuthority",
        };
        write!(f, "{}", s)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphNode {
    pub id: Uuid,
    pub node_type: NodeType,
    pub label: String,
    pub properties: HashMap<String, serde_json::Value>,
    pub created_at: DateTime<Utc>,
    pub trust_score: f64, // 0.0-1.0
}

impl GraphNode {
    pub fn new(node_type: NodeType, label: impl Into<String>) -> Self {
        Self {
            id: Uuid::new_v4(),
            node_type,
            label: label.into(),
            properties: HashMap::new(),
            created_at: Utc::now(),
            trust_score: 0.5,
        }
    }

    pub fn with_property(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        self.properties.insert(key.into(), value);
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EdgeType {
    PaidBy,       // Transaction -> BankAccount
    PaidTo,       // Transaction -> Merchant/Supplier
    HasInvoice,   // Transaction -> Invoice
    HasReceipt,   // Transaction -> Receipt
    HasVoucher,   // Transaction -> Voucher
    SupplierOf,   // Supplier -> Company
    EmployedBy,   // Person -> Company
    SubsidiaryOf, // Company -> Company
    SimilarTo,    // Merchant -> Merchant (alias-relationer)
    DuplicateOf,  // Catch-all för duplikat-detection
    RelatedTo,    // Generic
}

impl std::fmt::Display for EdgeType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            EdgeType::PaidBy => "PaidBy",
            EdgeType::PaidTo => "PaidTo",
            EdgeType::HasInvoice => "HasInvoice",
            EdgeType::HasReceipt => "HasReceipt",
            EdgeType::HasVoucher => "HasVoucher",
            EdgeType::SupplierOf => "SupplierOf",
            EdgeType::EmployedBy => "EmployedBy",
            EdgeType::SubsidiaryOf => "SubsidiaryOf",
            EdgeType::SimilarTo => "SimilarTo",
            EdgeType::DuplicateOf => "DuplicateOf",
            EdgeType::RelatedTo => "RelatedTo",
        };
        write!(f, "{}", s)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphEdge {
    pub id: Uuid,
    pub from: Uuid,
    pub to: Uuid,
    pub edge_type: EdgeType,
    pub weight: f64, // 0.0-1.0 – confidence i edge
    pub properties: HashMap<String, serde_json::Value>,
    pub created_at: DateTime<Utc>,
}

impl GraphEdge {
    pub fn new(from: Uuid, to: Uuid, edge_type: EdgeType, weight: f64) -> Self {
        Self {
            id: Uuid::new_v4(),
            from,
            to,
            edge_type,
            weight,
            properties: HashMap::new(),
            created_at: Utc::now(),
        }
    }

    pub fn with_property(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        self.properties.insert(key.into(), value);
        self
    }
}

pub struct EconomicGraph {
    nodes: HashMap<Uuid, GraphNode>,
    edges: HashMap<Uuid, GraphEdge>,
    /// adjacency: node_id -> Vec<edge_id>
    adjacency: HashMap<Uuid, Vec<Uuid>>,
    /// reverse adjacency
    reverse_adjacency: HashMap<Uuid, Vec<Uuid>>,
}

impl Default for EconomicGraph {
    fn default() -> Self {
        Self::new()
    }
}

impl EconomicGraph {
    pub fn new() -> Self {
        Self {
            nodes: HashMap::new(),
            edges: HashMap::new(),
            adjacency: HashMap::new(),
            reverse_adjacency: HashMap::new(),
        }
    }

    // ── Nodes ──────────────────────────────────────────────────────────────

    pub fn add_node(&mut self, node: GraphNode) -> Uuid {
        let id = node.id;
        self.adjacency.entry(id).or_default();
        self.reverse_adjacency.entry(id).or_default();
        self.nodes.insert(id, node);
        id
    }

    pub fn get_node(&self, id: &Uuid) -> Option<&GraphNode> {
        self.nodes.get(id)
    }

    pub fn update_trust(&mut self, id: &Uuid, score: f64) {
        if let Some(node) = self.nodes.get_mut(id) {
            node.trust_score = score.clamp(0.0, 1.0);
        }
    }

    pub fn nodes_of_type(&self, node_type: &NodeType) -> Vec<&GraphNode> {
        self.nodes
            .values()
            .filter(|n| &n.node_type == node_type)
            .collect()
    }

    // ── Edges ──────────────────────────────────────────────────────────────

    pub fn add_edge(&mut self, edge: GraphEdge) -> Uuid {
        let id = edge.id;
        let from = edge.from;
        let to = edge.to;

        // Ensure both endpoints exist in adjacency maps
        self.adjacency.entry(from).or_default().push(id);
        self.adjacency.entry(to).or_default();
        self.reverse_adjacency.entry(to).or_default().push(id);
        self.reverse_adjacency.entry(from).or_default();

        self.edges.insert(id, edge);
        id
    }

    pub fn edges_from(&self, node_id: &Uuid) -> Vec<&GraphEdge> {
        self.adjacency
            .get(node_id)
            .map(|ids| ids.iter().filter_map(|id| self.edges.get(id)).collect())
            .unwrap_or_default()
    }

    pub fn edges_to(&self, node_id: &Uuid) -> Vec<&GraphEdge> {
        self.reverse_adjacency
            .get(node_id)
            .map(|ids| ids.iter().filter_map(|id| self.edges.get(id)).collect())
            .unwrap_or_default()
    }

    pub fn edges_of_type(&self, edge_type: &EdgeType) -> Vec<&GraphEdge> {
        self.edges
            .values()
            .filter(|e| &e.edge_type == edge_type)
            .collect()
    }

    // ── Traversal ──────────────────────────────────────────────────────────

    /// All neighbor nodes reachable via outgoing edges from `node_id`
    pub fn neighbors(&self, node_id: &Uuid) -> Vec<&GraphNode> {
        self.edges_from(node_id)
            .iter()
            .filter_map(|e| self.nodes.get(&e.to))
            .collect()
    }

    /// BFS: find all simple paths from `from` to `to` up to `max_depth` hops.
    /// Returns each path as an ordered list of node IDs (inclusive of start/end).
    pub fn paths_between(&self, from: &Uuid, to: &Uuid, max_depth: u8) -> Vec<Vec<Uuid>> {
        if from == to {
            return vec![vec![*from]];
        }

        let mut results: Vec<Vec<Uuid>> = Vec::new();
        // BFS queue: (current_node, path_so_far)
        let mut queue: VecDeque<(Uuid, Vec<Uuid>)> = VecDeque::new();
        queue.push_back((*from, vec![*from]));

        while let Some((current, path)) = queue.pop_front() {
            if path.len() as u8 > max_depth {
                continue;
            }

            for edge in self.edges_from(&current) {
                let next = edge.to;

                if next == *to {
                    let mut full_path = path.clone();
                    full_path.push(next);
                    results.push(full_path);
                    continue;
                }

                // Avoid cycles
                if !path.contains(&next) && (path.len() as u8) < max_depth {
                    let mut new_path = path.clone();
                    new_path.push(next);
                    queue.push_back((next, new_path));
                }
            }
        }

        results
    }

    /// BFS connected component (undirected: follows both forward and reverse edges)
    pub fn connected_component(&self, node_id: &Uuid) -> HashSet<Uuid> {
        let mut visited: HashSet<Uuid> = HashSet::new();
        let mut queue: VecDeque<Uuid> = VecDeque::new();

        if !self.nodes.contains_key(node_id) {
            return visited;
        }

        queue.push_back(*node_id);
        visited.insert(*node_id);

        while let Some(current) = queue.pop_front() {
            // Forward edges
            for edge in self.edges_from(&current) {
                if visited.insert(edge.to) {
                    queue.push_back(edge.to);
                }
            }
            // Reverse edges
            for edge in self.edges_to(&current) {
                if visited.insert(edge.from) {
                    queue.push_back(edge.from);
                }
            }
        }

        visited
    }

    // ── Insights ──────────────────────────────────────────────────────────

    pub fn count_nodes(&self) -> usize {
        self.nodes.len()
    }

    pub fn count_edges(&self) -> usize {
        self.edges.len()
    }

    pub fn statistics(&self) -> GraphStatistics {
        let total_nodes = self.nodes.len();
        let total_edges = self.edges.len();

        let mut nodes_by_type: HashMap<String, usize> = HashMap::new();
        for node in self.nodes.values() {
            *nodes_by_type.entry(node.node_type.to_string()).or_insert(0) += 1;
        }

        let mut edges_by_type: HashMap<String, usize> = HashMap::new();
        for edge in self.edges.values() {
            *edges_by_type
                .entry(edge.edge_type.to_string())
                .or_insert(0) += 1;
        }

        let average_degree = if total_nodes == 0 {
            0.0
        } else {
            (total_edges * 2) as f64 / total_nodes as f64
        };

        GraphStatistics {
            total_nodes,
            total_edges,
            nodes_by_type,
            edges_by_type,
            average_degree,
        }
    }

    // ── Fraud / anomaly hints ──────────────────────────────────────────────

    /// Returns pairs of nodes connected by DuplicateOf or SimilarTo edges,
    /// along with the edge weight as a similarity score.
    pub fn potential_duplicates(&self) -> Vec<(Uuid, Uuid, f64)> {
        let mut results = Vec::new();

        for edge in self.edges.values() {
            match edge.edge_type {
                EdgeType::DuplicateOf | EdgeType::SimilarTo => {
                    results.push((edge.from, edge.to, edge.weight));
                }
                _ => {}
            }
        }

        // Sort by confidence desc
        results.sort_by(|a, b| b.2.partial_cmp(&a.2).unwrap_or(std::cmp::Ordering::Equal));
        results
    }

    /// Nodes with no edges at all (neither in- nor out-degree)
    pub fn isolated_nodes(&self) -> Vec<&GraphNode> {
        self.nodes
            .values()
            .filter(|node| {
                let out = self
                    .adjacency
                    .get(&node.id)
                    .map(|v| v.is_empty())
                    .unwrap_or(true);
                let r#in = self
                    .reverse_adjacency
                    .get(&node.id)
                    .map(|v| v.is_empty())
                    .unwrap_or(true);
                out && r#in
            })
            .collect()
    }

    /// Top-N nodes by total degree (in + out)
    pub fn high_degree_nodes(&self, top_n: usize) -> Vec<(&GraphNode, usize)> {
        let mut degrees: Vec<(&GraphNode, usize)> = self
            .nodes
            .values()
            .map(|node| {
                let out = self
                    .adjacency
                    .get(&node.id)
                    .map(|v| v.len())
                    .unwrap_or(0);
                let r#in = self
                    .reverse_adjacency
                    .get(&node.id)
                    .map(|v| v.len())
                    .unwrap_or(0);
                (node, out + r#in)
            })
            .collect();

        degrees.sort_by(|a, b| b.1.cmp(&a.1));
        degrees.truncate(top_n);
        degrees
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct GraphStatistics {
    pub total_nodes: usize,
    pub total_edges: usize,
    pub nodes_by_type: HashMap<String, usize>,
    pub edges_by_type: HashMap<String, usize>,
    pub average_degree: f64,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_graph() -> (EconomicGraph, Uuid, Uuid, Uuid) {
        let mut g = EconomicGraph::new();
        let a = g.add_node(GraphNode::new(NodeType::Transaction, "tx-1"));
        let b = g.add_node(GraphNode::new(NodeType::Merchant, "ICA"));
        let c = g.add_node(GraphNode::new(NodeType::Receipt, "receipt-1"));
        g.add_edge(GraphEdge::new(a, b, EdgeType::PaidTo, 1.0));
        g.add_edge(GraphEdge::new(a, c, EdgeType::HasReceipt, 1.0));
        (g, a, b, c)
    }

    #[test]
    fn test_basic_add_and_get() {
        let (g, a, b, _c) = make_graph();
        assert!(g.get_node(&a).is_some());
        assert_eq!(g.count_nodes(), 3);
        assert_eq!(g.count_edges(), 2);
        let neighbors = g.neighbors(&a);
        assert_eq!(neighbors.len(), 2);
    }

    #[test]
    fn test_paths_between() {
        let (g, a, _b, c) = make_graph();
        let paths = g.paths_between(&a, &c, 3);
        assert!(!paths.is_empty());
        assert_eq!(paths[0][0], a);
        assert_eq!(*paths[0].last().unwrap(), c);
    }

    #[test]
    fn test_connected_component() {
        let (g, a, b, c) = make_graph();
        let comp = g.connected_component(&a);
        assert!(comp.contains(&a));
        assert!(comp.contains(&b));
        assert!(comp.contains(&c));
    }

    #[test]
    fn test_isolated_nodes() {
        let mut g = EconomicGraph::new();
        let lone = g.add_node(GraphNode::new(NodeType::Person, "orphan"));
        let isolated = g.isolated_nodes();
        assert_eq!(isolated.len(), 1);
        assert_eq!(isolated[0].id, lone);
    }

    #[test]
    fn test_statistics() {
        let (g, _, _, _) = make_graph();
        let stats = g.statistics();
        assert_eq!(stats.total_nodes, 3);
        assert_eq!(stats.total_edges, 2);
    }

    #[test]
    fn test_potential_duplicates() {
        let mut g = EconomicGraph::new();
        let a = g.add_node(GraphNode::new(NodeType::Merchant, "ICA Maxi"));
        let b = g.add_node(GraphNode::new(NodeType::Merchant, "ICA"));
        g.add_edge(GraphEdge::new(a, b, EdgeType::SimilarTo, 0.9));
        let dups = g.potential_duplicates();
        assert_eq!(dups.len(), 1);
        assert!((dups[0].2 - 0.9).abs() < f64::EPSILON);
    }
}
