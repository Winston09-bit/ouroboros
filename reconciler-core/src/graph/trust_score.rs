use crate::graph::economic_graph::{EdgeType, EconomicGraph, GraphNode, NodeType};
use uuid::Uuid;

/// Trust Score Engine – 0.0–1.0 score per entity
pub struct TrustScoreEngine;

impl TrustScoreEngine {
    /// Beräkna trust score baserat på:
    /// - Evidence completeness (har transaktionen kvitto?)
    /// - VAT consistency
    /// - Historisk pålitlighet
    /// - Anslutning till verifierade noder
    pub fn score_node(graph: &EconomicGraph, node_id: &Uuid) -> f64 {
        let node = match graph.get_node(node_id) {
            Some(n) => n,
            None => return 0.0,
        };

        match node.node_type {
            NodeType::Merchant => {
                let t = Self::merchant_trust(graph, node_id);
                t.overall_score
            }
            NodeType::Supplier => {
                let t = Self::supplier_trust(graph, node_id);
                t.overall_score
            }
            NodeType::Transaction => Self::score_transaction(graph, node_id, node),
            NodeType::Invoice => Self::score_invoice(graph, node_id, node),
            NodeType::Company => Self::score_company(graph, node_id, node),
            // Leaf/reference entities start at the node's own stored trust_score
            _ => node.trust_score,
        }
    }

    // ── Transaction ────────────────────────────────────────────────────────

    fn score_transaction(graph: &EconomicGraph, node_id: &Uuid, node: &GraphNode) -> f64 {
        let out_edges = graph.edges_from(node_id);

        let has_receipt = out_edges.iter().any(|e| e.edge_type == EdgeType::HasReceipt);
        let has_invoice = out_edges.iter().any(|e| e.edge_type == EdgeType::HasInvoice);
        let has_voucher = out_edges.iter().any(|e| e.edge_type == EdgeType::HasVoucher);
        let has_paid_to = out_edges.iter().any(|e| e.edge_type == EdgeType::PaidTo);
        let has_paid_by = out_edges.iter().any(|e| e.edge_type == EdgeType::PaidBy);

        // Evidence completeness (0.0-1.0)
        let evidence_score = {
            let mut score = 0.0f64;
            if has_receipt {
                score += 0.35;
            }
            if has_invoice {
                score += 0.30;
            }
            if has_voucher {
                score += 0.20;
            }
            if has_paid_to {
                score += 0.10;
            }
            if has_paid_by {
                score += 0.05;
            }
            score.min(1.0)
        };

        // Check VAT consistency hint stored in properties
        let vat_ok = node
            .properties
            .get("vat_verified")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        let vat_score = if vat_ok { 1.0 } else { 0.5 };

        // Trust propagation: average trust of connected verified nodes
        let connected_trust: f64 = {
            let trusted_neighbors: Vec<f64> = out_edges
                .iter()
                .filter_map(|e| graph.get_node(&e.to))
                .map(|n| n.trust_score)
                .collect();

            if trusted_neighbors.is_empty() {
                0.5
            } else {
                trusted_neighbors.iter().sum::<f64>() / trusted_neighbors.len() as f64
            }
        };

        // Weighted combination
        evidence_score * 0.5 + vat_score * 0.2 + connected_trust * 0.3
    }

    // ── Invoice ────────────────────────────────────────────────────────────

    fn score_invoice(graph: &EconomicGraph, node_id: &Uuid, node: &GraphNode) -> f64 {
        // An invoice that's been matched to a transaction gets a boost
        let matched = graph
            .edges_to(node_id)
            .iter()
            .any(|e| e.edge_type == EdgeType::HasInvoice);

        let vat_ok = node
            .properties
            .get("vat_verified")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        let has_supplier = graph
            .edges_to(node_id)
            .iter()
            .any(|e| graph.get_node(&e.from).map(|n| n.node_type == NodeType::Supplier).unwrap_or(false));

        let mut score = 0.4f64;
        if matched {
            score += 0.3;
        }
        if vat_ok {
            score += 0.2;
        }
        if has_supplier {
            score += 0.1;
        }
        score.min(1.0)
    }

    // ── Company ────────────────────────────────────────────────────────────

    fn score_company(graph: &EconomicGraph, node_id: &Uuid, node: &GraphNode) -> f64 {
        let tax_authority_connected = graph
            .edges_from(node_id)
            .iter()
            .chain(graph.edges_to(node_id).iter())
            .any(|e| {
                let other = if e.from == *node_id { &e.to } else { &e.from };
                graph
                    .get_node(other)
                    .map(|n| n.node_type == NodeType::TaxAuthority)
                    .unwrap_or(false)
            });

        let registered = node
            .properties
            .get("org_nr")
            .map(|v| !v.as_str().unwrap_or("").is_empty())
            .unwrap_or(false);

        let mut score = 0.5f64;
        if tax_authority_connected {
            score += 0.3;
        }
        if registered {
            score += 0.2;
        }
        score.min(1.0)
    }

    // ── Merchant ───────────────────────────────────────────────────────────

    /// Score för en specifik merchant
    pub fn merchant_trust(graph: &EconomicGraph, merchant_node_id: &Uuid) -> MerchantTrust {
        // Find all transactions pointing to this merchant
        let transactions: Vec<&crate::graph::economic_graph::GraphEdge> = graph
            .edges_to(merchant_node_id)
            .into_iter()
            .filter(|e| e.edge_type == EdgeType::PaidTo)
            .collect();

        let transaction_count = transactions.len();

        let mut receipts_found = 0usize;
        let mut vat_ok_count = 0usize;
        let mut flags: Vec<String> = Vec::new();

        for tx_edge in &transactions {
            let tx_node_id = &tx_edge.from;
            let tx_out = graph.edges_from(tx_node_id);

            if tx_out.iter().any(|e| e.edge_type == EdgeType::HasReceipt) {
                receipts_found += 1;
            }

            if let Some(tx_node) = graph.get_node(tx_node_id) {
                if tx_node
                    .properties
                    .get("vat_verified")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false)
                {
                    vat_ok_count += 1;
                }
            }
        }

        let receipt_completeness = if transaction_count == 0 {
            0.0
        } else {
            receipts_found as f64 / transaction_count as f64
        };

        let vat_consistency = if transaction_count == 0 {
            0.0
        } else {
            vat_ok_count as f64 / transaction_count as f64
        };

        // Check for similar/duplicate merchant edges (alias risk)
        let alias_count = graph
            .edges_from(merchant_node_id)
            .iter()
            .chain(graph.edges_to(merchant_node_id).iter())
            .filter(|e| e.edge_type == EdgeType::SimilarTo || e.edge_type == EdgeType::DuplicateOf)
            .count();

        if alias_count > 0 {
            flags.push(format!("merchant_has_{}_alias_relations", alias_count));
        }
        if receipt_completeness < 0.5 {
            flags.push("low_receipt_coverage".to_string());
        }
        if vat_consistency < 0.7 && transaction_count > 0 {
            flags.push("low_vat_consistency".to_string());
        }

        // response_reliability: proxy via retrieval_ok property on merchant node
        let response_reliability = if let Some(node) = graph.get_node(merchant_node_id) {
            node.properties
                .get("retrieval_success_rate")
                .and_then(|v| v.as_f64())
                .unwrap_or(0.5)
        } else {
            0.5
        };

        // Weighted overall
        let overall_score = receipt_completeness * 0.4
            + vat_consistency * 0.3
            + response_reliability * 0.2
            + if alias_count == 0 { 0.1 } else { 0.0 };

        MerchantTrust {
            merchant_id: *merchant_node_id,
            overall_score: overall_score.clamp(0.0, 1.0),
            receipt_completeness,
            vat_consistency,
            response_reliability,
            transaction_count,
            flags,
        }
    }

    // ── Supplier ───────────────────────────────────────────────────────────

    /// Score för en supplier
    pub fn supplier_trust(graph: &EconomicGraph, supplier_node_id: &Uuid) -> SupplierTrust {
        let node = match graph.get_node(supplier_node_id) {
            Some(n) => n,
            None => {
                return SupplierTrust {
                    supplier_id: *supplier_node_id,
                    overall_score: 0.0,
                    invoice_quality: 0.0,
                    vat_status_verified: false,
                    duplicate_invoice_risk: 1.0,
                    payment_reliability: 0.0,
                    flags: vec!["supplier_node_not_found".to_string()],
                }
            }
        };

        // Invoices issued by this supplier (SupplierOf edges → traverse to invoices)
        let invoices: Vec<Uuid> = graph
            .edges_from(supplier_node_id)
            .iter()
            .filter_map(|e| {
                if let Some(target) = graph.get_node(&e.to) {
                    if target.node_type == NodeType::Invoice {
                        return Some(e.to);
                    }
                }
                None
            })
            .collect();

        let invoice_count = invoices.len();

        // Invoice quality: fraction that are matched to transactions
        let matched_invoices = invoices
            .iter()
            .filter(|inv_id| {
                graph
                    .edges_to(inv_id)
                    .iter()
                    .any(|e| e.edge_type == EdgeType::HasInvoice)
            })
            .count();

        let invoice_quality = if invoice_count == 0 {
            0.5 // no data → neutral
        } else {
            matched_invoices as f64 / invoice_count as f64
        };

        // VAT status from property
        let vat_status_verified = node
            .properties
            .get("vat_registered")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        // Duplicate invoice risk: DuplicateOf edges among this supplier's invoices
        let duplicate_pairs = invoices
            .iter()
            .flat_map(|inv_id| graph.edges_from(inv_id))
            .filter(|e| e.edge_type == EdgeType::DuplicateOf)
            .count();

        let duplicate_invoice_risk = if invoice_count == 0 {
            0.0
        } else {
            (duplicate_pairs as f64 / invoice_count as f64).min(1.0)
        };

        // Payment reliability from property
        let payment_reliability = node
            .properties
            .get("payment_on_time_rate")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.5);

        let mut flags: Vec<String> = Vec::new();
        if !vat_status_verified {
            flags.push("vat_not_verified".to_string());
        }
        if duplicate_invoice_risk > 0.1 {
            flags.push(format!(
                "duplicate_invoice_risk_{:.0}pct",
                duplicate_invoice_risk * 100.0
            ));
        }
        if invoice_quality < 0.5 && invoice_count > 0 {
            flags.push("low_invoice_match_rate".to_string());
        }

        let overall_score = invoice_quality * 0.35
            + if vat_status_verified { 0.25 } else { 0.0 }
            + (1.0 - duplicate_invoice_risk) * 0.25
            + payment_reliability * 0.15;

        SupplierTrust {
            supplier_id: *supplier_node_id,
            overall_score: overall_score.clamp(0.0, 1.0),
            invoice_quality,
            vat_status_verified,
            duplicate_invoice_risk,
            payment_reliability,
            flags,
        }
    }
}

// ── Output structs ─────────────────────────────────────────────────────────

#[derive(Debug, Clone, serde::Serialize)]
pub struct MerchantTrust {
    pub merchant_id: Uuid,
    pub overall_score: f64,
    pub receipt_completeness: f64,
    pub vat_consistency: f64,
    pub response_reliability: f64,
    pub transaction_count: usize,
    pub flags: Vec<String>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct SupplierTrust {
    pub supplier_id: Uuid,
    pub overall_score: f64,
    pub invoice_quality: f64,
    pub vat_status_verified: bool,
    pub duplicate_invoice_risk: f64,
    pub payment_reliability: f64,
    pub flags: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::economic_graph::{EdgeType, EconomicGraph, GraphEdge, GraphNode, NodeType};

    #[test]
    fn test_merchant_trust_full_coverage() {
        let mut g = EconomicGraph::new();
        let merchant = g.add_node(GraphNode::new(NodeType::Merchant, "ICA"));
        let tx = g.add_node({
            let mut n = GraphNode::new(NodeType::Transaction, "tx-1");
            n.properties
                .insert("vat_verified".into(), serde_json::Value::Bool(true));
            n
        });
        let receipt = g.add_node(GraphNode::new(NodeType::Receipt, "receipt-1"));
        g.add_edge(GraphEdge::new(tx, merchant, EdgeType::PaidTo, 1.0));
        g.add_edge(GraphEdge::new(tx, receipt, EdgeType::HasReceipt, 1.0));

        let trust = TrustScoreEngine::merchant_trust(&g, &merchant);
        assert_eq!(trust.transaction_count, 1);
        assert!((trust.receipt_completeness - 1.0).abs() < f64::EPSILON);
        assert!((trust.vat_consistency - 1.0).abs() < f64::EPSILON);
        assert!(trust.overall_score > 0.5);
        assert!(trust.flags.is_empty());
    }

    #[test]
    fn test_supplier_trust_with_vat() {
        let mut g = EconomicGraph::new();
        let supplier = g.add_node({
            let mut n = GraphNode::new(NodeType::Supplier, "Acme AB");
            n.properties
                .insert("vat_registered".into(), serde_json::Value::Bool(true));
            n.properties.insert(
                "payment_on_time_rate".into(),
                serde_json::Value::from(0.9f64),
            );
            n
        });
        let trust = TrustScoreEngine::supplier_trust(&g, &supplier);
        assert!(trust.vat_status_verified);
        assert!(trust.overall_score > 0.0);
        assert!(!trust.flags.contains(&"vat_not_verified".to_string()));
    }
}
