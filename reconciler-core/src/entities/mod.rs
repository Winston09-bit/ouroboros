// src/entities/mod.rs — Multi-Entity Engine
// Hantering av holdingstrukturer, dotterbolag och intercompany reconciliation.

use std::collections::{HashMap, HashSet};
use rust_decimal::Decimal;
use rust_decimal::prelude::Zero;
use uuid::Uuid;

// ─── EntityType ───────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EntityType {
    HoldingCompany,
    OperatingSubsidiary,
    SPV,
    Branch,
    JointVenture,
}

impl EntityType {
    pub fn label(&self) -> &'static str {
        match self {
            EntityType::HoldingCompany       => "Holdingbolag",
            EntityType::OperatingSubsidiary  => "Rörelsedotterbolag",
            EntityType::SPV                  => "SPV",
            EntityType::Branch               => "Filial",
            EntityType::JointVenture         => "Joint Venture",
        }
    }

    /// Ska entitetens transaktioner elimineras vid konsolidering?
    pub fn is_consolidatable(&self) -> bool {
        !matches!(self, EntityType::JointVenture)
    }
}

// ─── Entity ───────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct Entity {
    pub id: Uuid,
    pub name: String,
    pub org_number: String,
    pub jurisdiction: String,
    pub currency: String,
    pub entity_type: EntityType,
    pub parent_id: Option<Uuid>,
    pub children: Vec<Entity>,
}

impl Entity {
    pub fn new(
        name: impl Into<String>,
        org_number: impl Into<String>,
        jurisdiction: impl Into<String>,
        currency: impl Into<String>,
        entity_type: EntityType,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            name: name.into(),
            org_number: org_number.into(),
            jurisdiction: jurisdiction.into(),
            currency: currency.into(),
            entity_type,
            parent_id: None,
            children: vec![],
        }
    }

    pub fn with_id(mut self, id: Uuid) -> Self {
        self.id = id;
        self
    }

    pub fn with_parent(mut self, parent_id: Uuid) -> Self {
        self.parent_id = Some(parent_id);
        self
    }

    pub fn add_child(&mut self, mut child: Entity) {
        child.parent_id = Some(self.id);
        self.children.push(child);
    }

    /// Returnerar en flat lista med alla entiteter i trädet (DFS).
    pub fn all_entities(&self) -> Vec<&Entity> {
        let mut result = vec![self];
        for child in &self.children {
            result.extend(child.all_entities());
        }
        result
    }

    /// Returnerar total antal entiteter inklusive sig själv.
    pub fn count(&self) -> usize {
        1 + self.children.iter().map(|c| c.count()).sum::<usize>()
    }

    /// Unika jurisdiktioner i trädet.
    pub fn jurisdictions(&self) -> Vec<String> {
        let mut seen = HashSet::new();
        let mut result = vec![];
        for e in self.all_entities() {
            if seen.insert(e.jurisdiction.clone()) {
                result.push(e.jurisdiction.clone());
            }
        }
        result.sort();
        result
    }
}

// ─── EntityTree ───────────────────────────────────────────────────────────────

#[derive(Debug)]
pub struct EntityTree {
    pub root: Entity,
    pub total_entities: usize,
    pub jurisdictions: Vec<String>,
}

impl EntityTree {
    fn from_root(root: Entity) -> Self {
        let total_entities = root.count();
        let jurisdictions = root.jurisdictions();
        Self { root, total_entities, jurisdictions }
    }
}

// ─── Transaction ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct Transaction {
    pub id: Uuid,
    pub entity_id: Uuid,
    /// Motpartsbolagets ID vid intercompany-transaktioner.
    pub counterparty_entity_id: Option<Uuid>,
    pub amount: Decimal,
    pub currency: String,
    pub description: String,
    pub account_debit: String,
    pub account_credit: String,
    pub period: String,       // "YYYY-MM"
    pub is_intercompany: bool,
    pub direction: TransactionDirection,
}

impl Transaction {
    pub fn new(
        entity_id: Uuid,
        amount: Decimal,
        currency: impl Into<String>,
        description: impl Into<String>,
        period: impl Into<String>,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            entity_id,
            counterparty_entity_id: None,
            amount,
            currency: currency.into(),
            description: description.into(),
            account_debit: String::new(),
            account_credit: String::new(),
            period: period.into(),
            is_intercompany: false,
            direction: TransactionDirection::Debit,
        }
    }

    pub fn as_intercompany(
        mut self,
        counterparty_id: Uuid,
        direction: TransactionDirection,
    ) -> Self {
        self.counterparty_entity_id = Some(counterparty_id);
        self.is_intercompany = true;
        self.direction = direction;
        self
    }

    pub fn with_accounts(mut self, debit: impl Into<String>, credit: impl Into<String>) -> Self {
        self.account_debit = debit.into();
        self.account_credit = credit.into();
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TransactionDirection {
    Debit,
    Credit,
}

// ─── EntityReport ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct EntityReport {
    pub entity_id: Uuid,
    pub entity_name: String,
    pub period: String,
    pub revenue: Decimal,
    pub expenses: Decimal,
    pub assets: Decimal,
    pub liabilities: Decimal,
    pub intercompany_balance: Decimal,
    pub transactions: Vec<Transaction>,
}

impl EntityReport {
    fn net_income(&self) -> Decimal {
        self.revenue - self.expenses
    }
}

// ─── ConsolidatedReport ───────────────────────────────────────────────────────

#[derive(Debug)]
pub struct ConsolidatedReport {
    pub period: String,
    pub entities: Vec<EntityReport>,
    pub intercompany_eliminations: Vec<Transaction>,
    pub consolidated_revenue: Decimal,
    pub consolidated_expenses: Decimal,
    pub consolidated_assets: Decimal,
    pub consolidated_liabilities: Decimal,
    pub consolidated_net_income: Decimal,
}

impl ConsolidatedReport {
    pub fn elimination_volume(&self) -> Decimal {
        self.intercompany_eliminations
            .iter()
            .fold(Decimal::zero(), |acc, t| acc + t.amount)
    }

    pub fn entity_count(&self) -> usize {
        self.entities.len()
    }
}

// ─── IntercompanyPair ─────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct IntercompanyPair {
    pub debit_txn: Transaction,
    pub credit_txn: Transaction,
    pub delta: Decimal,    // Skillnad i belopp (normalt 0 vid korrekt bokföring)
    pub is_balanced: bool,
}

impl IntercompanyPair {
    fn new(debit: Transaction, credit: Transaction) -> Self {
        let delta = (debit.amount - credit.amount).abs();
        let is_balanced = delta == Decimal::zero();
        Self { debit_txn: debit, credit_txn: credit, delta, is_balanced }
    }
}

// ─── IntercompanyResult ───────────────────────────────────────────────────────

#[derive(Debug)]
pub struct IntercompanyResult {
    pub matched_pairs: Vec<IntercompanyPair>,
    pub unmatched: Vec<Transaction>,
    pub total_intercompany_volume: Decimal,
    pub unbalanced_count: usize,
}

impl IntercompanyResult {
    pub fn is_fully_balanced(&self) -> bool {
        self.unmatched.is_empty() && self.unbalanced_count == 0
    }
}

// ─── EntityRegistry ──────────────────────────────────────────────────────────
// Intern hjälpstruktur för att bygga träd från flat lista.

struct EntityRegistry {
    entities: HashMap<Uuid, Entity>,
}

impl EntityRegistry {
    fn new() -> Self { Self { entities: HashMap::new() } }

    fn insert(&mut self, entity: Entity) {
        self.entities.insert(entity.id, entity);
    }

    fn build_tree(mut self, root_id: Uuid) -> Option<Entity> {
        // Bygg ett rekursivt träd inifrån och ut.
        fn attach_children(
            id: Uuid,
            all: &mut HashMap<Uuid, Entity>,
        ) -> Option<Entity> {
            let mut node = all.remove(&id)?;
            let child_ids: Vec<Uuid> = all
                .values()
                .filter(|e| e.parent_id == Some(id))
                .map(|e| e.id)
                .collect();
            for child_id in child_ids {
                if let Some(child) = attach_children(child_id, all) {
                    node.children.push(child);
                }
            }
            Some(node)
        }
        attach_children(root_id, &mut self.entities)
    }
}

// ─── EntityEngine ─────────────────────────────────────────────────────────────

pub struct EntityEngine {
    /// Org → root Entity.
    entity_trees: HashMap<String, Entity>,
    /// Org → Transactions (flat lista).
    transactions: HashMap<String, Vec<Transaction>>,
    /// Org → EntityReport map (pre-computed).
    entity_reports: HashMap<Uuid, EntityReport>,
}

impl EntityEngine {
    pub fn new() -> Self {
        Self {
            entity_trees: HashMap::new(),
            transactions: HashMap::new(),
            entity_reports: HashMap::new(),
        }
    }

    // ── Setup ──────────────────────────────────────────────────────────────────

    /// Registrera ett entitetsträd för en organisation.
    pub fn register_entity_tree(&mut self, org_id: impl Into<String>, root: Entity) {
        self.entity_trees.insert(org_id.into(), root);
    }

    /// Ladda transaktioner för en organisation.
    pub fn load_transactions(&mut self, org_id: impl Into<String>, txns: Vec<Transaction>) {
        let org_id = org_id.into();
        for txn in &txns {
            let entity_id = txn.entity_id;
            let report = self.entity_reports.entry(entity_id).or_insert_with(|| EntityReport {
                entity_id,
                entity_name: String::new(),
                period: txn.period.clone(),
                revenue: Decimal::zero(),
                expenses: Decimal::zero(),
                assets: Decimal::zero(),
                liabilities: Decimal::zero(),
                intercompany_balance: Decimal::zero(),
                transactions: vec![],
            });
            // Enkel klassificering baserat på kontonummer (BAS-kontoplan-liknande).
            Self::classify_transaction_into_report(report, txn);
        }
        self.transactions.insert(org_id, txns);
    }

    fn classify_transaction_into_report(report: &mut EntityReport, txn: &Transaction) {
        report.transactions.push(txn.clone());
        if txn.is_intercompany {
            match txn.direction {
                TransactionDirection::Credit => report.intercompany_balance += txn.amount,
                TransactionDirection::Debit  => report.intercompany_balance -= txn.amount,
            }
            return;
        }
        // Klassificering baserat på BAS-kontoplan:
        // Intäkt: kredit på 3xxx-konto
        // Kostnad: debit på 4-7xxx-konto
        // Tillgång: debit på 1xxx-konto
        // Skuld: kredit på 2xxx-konto
        let credit_first = txn.account_credit
            .trim_start_matches('0')
            .chars()
            .next();
        let debit_first = txn.account_debit
            .trim_start_matches('0')
            .chars()
            .next();

        // Intäkt: kreditkonto 3xxx
        if credit_first == Some('3') {
            report.revenue += txn.amount;
            return;
        }
        // Kostnad: debitkonto 4-7xxx
        if matches!(debit_first, Some('4') | Some('5') | Some('6') | Some('7')) {
            report.expenses += txn.amount;
            return;
        }
        // Tillgång: debitkonto 1xxx
        if debit_first == Some('1') {
            report.assets += txn.amount;
            return;
        }
        // Skuld: kreditkonto 2xxx
        if credit_first == Some('2') {
            report.liabilities += txn.amount;
            return;
        }
        // Fallback
        report.expenses += txn.amount;
    }

    // ── Public API ─────────────────────────────────────────────────────────────

    /// Hämta entitetsträdet för en organisation.
    pub fn get_entity_tree(&self, org_id: &str) -> EntityTree {
        match self.entity_trees.get(org_id) {
            Some(root) => EntityTree::from_root(root.clone()),
            None => EntityTree {
                root: Entity::new(
                    format!("Okänd org: {org_id}"),
                    "",
                    "SE",
                    "SEK",
                    EntityType::HoldingCompany,
                ),
                total_entities: 0,
                jurisdictions: vec![],
            },
        }
    }

    /// Konsoliderad rapport över hela koncernen för given period.
    pub fn consolidated_report(&self, org_id: &str, period: &str) -> ConsolidatedReport {
        let tree = self.get_entity_tree(org_id);
        let all_entities: Vec<&Entity> = tree.root.all_entities();
        let _entity_ids: HashSet<Uuid> = all_entities.iter().map(|e| e.id).collect();

        let txns = self.transactions.get(org_id).cloned().unwrap_or_default();
        let period_txns: Vec<&Transaction> = txns.iter()
            .filter(|t| t.period.starts_with(period))
            .collect();

        // Bygg per-entitetsrapporter för perioden.
        let mut entity_reports: HashMap<Uuid, EntityReport> = HashMap::new();
        for e in &all_entities {
            entity_reports.insert(e.id, EntityReport {
                entity_id: e.id,
                entity_name: e.name.clone(),
                period: period.into(),
                revenue: Decimal::zero(),
                expenses: Decimal::zero(),
                assets: Decimal::zero(),
                liabilities: Decimal::zero(),
                intercompany_balance: Decimal::zero(),
                transactions: vec![],
            });
        }
        for txn in &period_txns {
            if let Some(report) = entity_reports.get_mut(&txn.entity_id) {
                Self::classify_transaction_into_report(report, txn);
            }
        }

        // Eliminera intercompany-transaktioner.
        let intercompany_txns: Vec<Transaction> = period_txns
            .iter()
            .filter(|t| t.is_intercompany)
            .map(|t| (*t).clone())
            .collect();
        let eliminations = self.eliminate_intercompany(&intercompany_txns);

        // Summera konsoliderade tal.
        let mut consolidated_revenue   = Decimal::zero();
        let mut consolidated_expenses  = Decimal::zero();
        let mut consolidated_assets    = Decimal::zero();
        let mut consolidated_liabilities = Decimal::zero();

        for report in entity_reports.values() {
            consolidated_revenue     += report.revenue;
            consolidated_expenses    += report.expenses;
            consolidated_assets      += report.assets;
            consolidated_liabilities += report.liabilities;
        }

        // Dra av eliminerade belopp.
        for elim in &eliminations {
            consolidated_revenue   -= elim.amount / Decimal::from(2);
            consolidated_expenses  -= elim.amount / Decimal::from(2);
        }

        let consolidated_net_income = consolidated_revenue - consolidated_expenses;

        let mut reports: Vec<EntityReport> = entity_reports.into_values().collect();
        reports.sort_by(|a, b| a.entity_name.cmp(&b.entity_name));

        ConsolidatedReport {
            period: period.into(),
            entities: reports,
            intercompany_eliminations: eliminations,
            consolidated_revenue,
            consolidated_expenses,
            consolidated_assets,
            consolidated_liabilities,
            consolidated_net_income,
        }
    }

    /// Matcha intercompany-transaktioner mot varandra.
    pub fn intercompany_reconciliation(
        &self,
        entities: &[Entity],
    ) -> IntercompanyResult {
        let entity_ids: HashSet<Uuid> = entities.iter().map(|e| e.id).collect();

        // Samla alla IC-transaktioner för dessa entiteter.
        let ic_txns: Vec<Transaction> = self.transactions.values()
            .flat_map(|v| v.iter())
            .filter(|t| t.is_intercompany && entity_ids.contains(&t.entity_id))
            .cloned()
            .collect();

        let mut matched: Vec<IntercompanyPair> = vec![];
        let mut unmatched: Vec<Transaction> = vec![];
        let mut used: HashSet<Uuid> = HashSet::new();
        let mut total_volume = Decimal::zero();

        // Debit-transaktioner söker sin matchande credit hos motparten.
        let debits: Vec<Transaction> = ic_txns.iter()
            .filter(|t| t.direction == TransactionDirection::Debit)
            .cloned()
            .collect();

        for debit in &debits {
            if used.contains(&debit.id) { continue; }

            let match_opt = ic_txns.iter().find(|t| {
                !used.contains(&t.id)
                    && t.direction == TransactionDirection::Credit
                    && t.entity_id == debit.counterparty_entity_id.unwrap_or(Uuid::nil())
                    && t.counterparty_entity_id == Some(debit.entity_id)
                    && t.period == debit.period
                    && (t.amount - debit.amount).abs() < Decimal::new(1, 2)
            });

            if let Some(credit) = match_opt {
                used.insert(debit.id);
                used.insert(credit.id);
                total_volume += debit.amount;
                let pair = IntercompanyPair::new(debit.clone(), credit.clone());
                matched.push(pair);
            }
        }

        for txn in &ic_txns {
            if !used.contains(&txn.id) {
                unmatched.push(txn.clone());
                total_volume += txn.amount;
            }
        }

        let unbalanced_count = matched.iter().filter(|p| !p.is_balanced).count();

        IntercompanyResult {
            matched_pairs: matched,
            unmatched,
            total_intercompany_volume: total_volume,
            unbalanced_count,
        }
    }

    /// Eliminera intercompany-transaktioner (tar bort matchade par).
    pub fn eliminate_intercompany(&self, txns: &[Transaction]) -> Vec<Transaction> {
        let mut used: HashSet<Uuid> = HashSet::new();
        let mut eliminations: Vec<Transaction> = vec![];

        let debits: Vec<&Transaction> = txns.iter()
            .filter(|t| t.direction == TransactionDirection::Debit)
            .collect();

        for debit in debits.iter() {
            if used.contains(&debit.id) { continue; }

            let match_opt = txns.iter().find(|t| {
                !used.contains(&t.id)
                    && t.direction == TransactionDirection::Credit
                    && t.entity_id == debit.counterparty_entity_id.unwrap_or(Uuid::nil())
                    && t.counterparty_entity_id == Some(debit.entity_id)
            });

            if let Some(credit) = match_opt {
                used.insert(debit.id);
                used.insert(credit.id);
                // Skapa en elimineringsspost.
                let mut elim: Transaction = (*debit).clone();
                elim.id = Uuid::new_v4();
                elim.description = format!(
                    "[ELIMINERING] {} ↔ {} ({})",
                    debit.entity_id,
                    credit.entity_id,
                    debit.period
                );
                elim.amount = (debit.amount + credit.amount) / Decimal::from(2);
                eliminations.push(elim);
            }
        }

        eliminations
    }
}

impl Default for EntityEngine {
    fn default() -> Self { Self::new() }
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal::prelude::FromStr;

    // ── Hjälpfunktioner ───────────────────────────────────────────────────────

    fn holding() -> Entity {
        Entity::new("HoldingBolag AB", "556900-0001", "SE", "SEK", EntityType::HoldingCompany)
    }

    fn subsidiary(name: &str, org: &str) -> Entity {
        Entity::new(name, org, "SE", "SEK", EntityType::OperatingSubsidiary)
    }

    fn spv(name: &str) -> Entity {
        Entity::new(name, "556900-0099", "SE", "SEK", EntityType::SPV)
    }

    fn build_tree() -> Entity {
        let mut root = holding();
        let sub1 = subsidiary("Dotter AB", "556900-0002");
        let sub2 = subsidiary("Syster AB", "556900-0003");
        root.add_child(sub1);
        root.add_child(sub2);
        root
    }

    fn ic_debit(from: Uuid, to: Uuid, amount: &str, period: &str) -> Transaction {
        Transaction::new(from, Decimal::from_str(amount).unwrap(), "SEK", "IC lån", period)
            .as_intercompany(to, TransactionDirection::Debit)
    }

    fn ic_credit(from: Uuid, to: Uuid, amount: &str, period: &str) -> Transaction {
        Transaction::new(from, Decimal::from_str(amount).unwrap(), "SEK", "IC lån motpart", period)
            .as_intercompany(to, TransactionDirection::Credit)
    }

    // ── Entitetsträd ──────────────────────────────────────────────────────────

    #[test]
    fn entity_tree_count_is_correct() {
        let root = build_tree();
        assert_eq!(root.count(), 3);
    }

    #[test]
    fn entity_tree_jurisdictions() {
        let root = build_tree();
        let j = root.jurisdictions();
        assert_eq!(j, vec!["SE"]);
    }

    #[test]
    fn multi_jurisdiction_tree() {
        let mut root = holding();
        let mut sub = subsidiary("Nordic AB", "556900-0010");
        let foreign = Entity::new("Foreign Ltd", "IE12345", "IE", "EUR", EntityType::Branch);
        sub.add_child(foreign);
        root.add_child(sub);
        let j = root.jurisdictions();
        assert!(j.contains(&"SE".to_string()));
        assert!(j.contains(&"IE".to_string()));
        assert_eq!(j.len(), 2);
    }

    // ── EntityEngine: träd-registrering ──────────────────────────────────────

    #[test]
    fn get_entity_tree_returns_registered_tree() {
        let mut engine = EntityEngine::new();
        let root = build_tree();
        engine.register_entity_tree("org-1", root.clone());
        let tree = engine.get_entity_tree("org-1");
        assert_eq!(tree.total_entities, 3);
        assert_eq!(tree.root.name, root.name);
    }

    #[test]
    fn get_entity_tree_unknown_org_returns_empty() {
        let engine = EntityEngine::new();
        let tree = engine.get_entity_tree("nonexistent");
        assert_eq!(tree.total_entities, 0);
    }

    // ── Intercompany reconciliation ───────────────────────────────────────────

    #[test]
    fn matched_intercompany_pair_is_balanced() {
        let entity_a = Uuid::new_v4();
        let entity_b = Uuid::new_v4();

        let debit  = ic_debit(entity_a, entity_b, "50000", "2024-01");
        let credit = ic_credit(entity_b, entity_a, "50000", "2024-01");

        let pair = IntercompanyPair::new(debit, credit);
        assert!(pair.is_balanced);
        assert_eq!(pair.delta, Decimal::zero());
    }

    #[test]
    fn unmatched_ic_transaction_flagged() {
        let entity_a = Uuid::new_v4();
        let entity_b = Uuid::new_v4();

        let debit  = ic_debit(entity_a, entity_b, "50000", "2024-01");
        // credit med fel belopp
        let credit = ic_credit(entity_b, entity_a, "49000", "2024-01");

        let pair = IntercompanyPair::new(debit, credit);
        assert!(!pair.is_balanced);
        assert_eq!(pair.delta, Decimal::from(1000));
    }

    #[test]
    fn intercompany_reconciliation_finds_matching_pair() {
        let mut engine = EntityEngine::new();
        let entity_a_id = Uuid::new_v4();
        let entity_b_id = Uuid::new_v4();

        let ea = Entity::new("A AB", "556900-A", "SE", "SEK", EntityType::HoldingCompany)
            .with_id(entity_a_id);
        let eb = Entity::new("B AB", "556900-B", "SE", "SEK", EntityType::OperatingSubsidiary)
            .with_id(entity_b_id);

        let debit  = ic_debit(entity_a_id, entity_b_id, "100000", "2024-03");
        let credit = ic_credit(entity_b_id, entity_a_id, "100000", "2024-03");

        engine.load_transactions("org-1", vec![debit, credit]);

        let entities = vec![ea, eb];
        let result = engine.intercompany_reconciliation(&entities);

        assert_eq!(result.matched_pairs.len(), 1);
        assert!(result.unmatched.is_empty());
        assert!(result.is_fully_balanced());
        assert_eq!(result.total_intercompany_volume, Decimal::from(100_000));
    }

    #[test]
    fn intercompany_reconciliation_flags_unmatched() {
        let mut engine = EntityEngine::new();
        let entity_a_id = Uuid::new_v4();
        let entity_b_id = Uuid::new_v4();

        let ea = Entity::new("A AB", "556900-A", "SE", "SEK", EntityType::HoldingCompany)
            .with_id(entity_a_id);
        let eb = Entity::new("B AB", "556900-B", "SE", "SEK", EntityType::OperatingSubsidiary)
            .with_id(entity_b_id);

        // Bara debit, ingen matchande credit.
        let debit = ic_debit(entity_a_id, entity_b_id, "75000", "2024-03");
        engine.load_transactions("org-1", vec![debit]);

        let entities = vec![ea, eb];
        let result = engine.intercompany_reconciliation(&entities);

        assert!(result.matched_pairs.is_empty());
        assert_eq!(result.unmatched.len(), 1);
        assert!(!result.is_fully_balanced());
    }

    // ── eliminate_intercompany ────────────────────────────────────────────────

    #[test]
    fn eliminate_intercompany_removes_matched_pairs() {
        let engine = EntityEngine::new();
        let a = Uuid::new_v4();
        let b = Uuid::new_v4();

        let debit  = ic_debit(a, b, "200000", "2024-06");
        let credit = ic_credit(b, a, "200000", "2024-06");

        let eliminations = engine.eliminate_intercompany(&[debit, credit]);
        assert_eq!(eliminations.len(), 1);
        assert_eq!(eliminations[0].amount, Decimal::from(200_000));
        assert!(eliminations[0].description.contains("[ELIMINERING]"));
    }

    #[test]
    fn eliminate_intercompany_no_match_returns_empty() {
        let engine = EntityEngine::new();
        let a = Uuid::new_v4();
        let b = Uuid::new_v4();
        let c = Uuid::new_v4();

        // Debit mot c, men credit mot b – ingen match.
        let debit  = ic_debit(a, c, "10000", "2024-06");
        let credit = ic_credit(b, a, "10000", "2024-06");

        let eliminations = engine.eliminate_intercompany(&[debit, credit]);
        assert!(eliminations.is_empty());
    }

    // ── ConsolidatedReport ────────────────────────────────────────────────────

    #[test]
    fn consolidated_report_sums_revenue_and_expenses() {
        let mut engine = EntityEngine::new();

        let root_id = Uuid::new_v4();
        let sub_id  = Uuid::new_v4();

        let root = Entity::new("Root AB", "556900-R", "SE", "SEK", EntityType::HoldingCompany)
            .with_id(root_id);
        let mut parent = root.clone();
        let sub = Entity::new("Sub AB", "556900-S", "SE", "SEK", EntityType::OperatingSubsidiary)
            .with_id(sub_id)
            .with_parent(root_id);
        parent.add_child(sub);

        engine.register_entity_tree("org-1", parent);

        let revenue_txn = Transaction::new(
            root_id,
            Decimal::from(100_000),
            "SEK",
            "Försäljning",
            "2024-01",
        ).with_accounts("1510", "3000");

        let expense_txn = Transaction::new(
            sub_id,
            Decimal::from(40_000),
            "SEK",
            "Lönekostnad",
            "2024-01",
        ).with_accounts("7210", "2710");

        engine.load_transactions("org-1", vec![revenue_txn, expense_txn]);

        let report = engine.consolidated_report("org-1", "2024-01");

        // Intäkt från root, kostnad från sub.
        assert_eq!(report.consolidated_revenue, Decimal::from(100_000));
        assert_eq!(report.consolidated_expenses, Decimal::from(40_000));
        assert_eq!(report.consolidated_net_income, Decimal::from(60_000));
        assert_eq!(report.entity_count(), 2);
    }

    // ── EntityType helpers ────────────────────────────────────────────────────

    #[test]
    fn entity_type_labels() {
        assert_eq!(EntityType::HoldingCompany.label(), "Holdingbolag");
        assert_eq!(EntityType::SPV.label(), "SPV");
        assert_eq!(EntityType::JointVenture.label(), "Joint Venture");
    }

    #[test]
    fn joint_venture_not_consolidatable() {
        assert!(!EntityType::JointVenture.is_consolidatable());
        assert!(EntityType::OperatingSubsidiary.is_consolidatable());
    }
}
