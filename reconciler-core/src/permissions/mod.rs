// src/permissions/mod.rs — RBAC + ABAC Permission System
// Enterprise-grade behörighetssystem för Reconciler

use std::collections::HashMap;
use std::sync::Arc;
use rust_decimal::Decimal;

// ─── Subject ─────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct Subject {
    pub user_id: String,
    pub org_id: String,
    pub roles: Vec<Role>,
    /// Vilka entity-IDs (bolag/dotterbolag) användaren har explicit access till.
    pub entity_access: Vec<String>,
}

impl Subject {
    pub fn new(user_id: impl Into<String>, org_id: impl Into<String>) -> Self {
        Self {
            user_id: user_id.into(),
            org_id: org_id.into(),
            roles: vec![],
            entity_access: vec![],
        }
    }

    pub fn with_roles(mut self, roles: Vec<Role>) -> Self {
        self.roles = roles;
        self
    }

    pub fn with_entity_access(mut self, entities: Vec<String>) -> Self {
        self.entity_access = entities;
        self
    }

    /// Returnerar true om subjektet har någon av de givna rollerna.
    pub fn has_any_role(&self, roles: &[Role]) -> bool {
        self.roles.iter().any(|r| roles.contains(r))
    }

    /// Returnerar det högsta approval-taket (SEK). None = obegränsat.
    pub fn max_approval_limit(&self) -> Option<Decimal> {
        if self.has_unlimited_approval() {
            return None;
        }
        self.roles
            .iter()
            .filter_map(|r| r.approval_ceiling())
            .max()
    }

    /// Returnerar true om någon roll ger obegränsat godkännande.
    pub fn has_unlimited_approval(&self) -> bool {
        self.roles.iter().any(|r| matches!(r, Role::SuperAdmin | Role::OrgAdmin))
    }
}

// ─── Role ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Role {
    SuperAdmin,
    OrgAdmin,
    CFO,
    AccountManager,
    Bookkeeper,
    Auditor,     // read-only + audit pack export
    APIClient,
    ReadOnly,
}

impl Role {
    /// Penninggräns för godkännande (SEK). None = rollen kan inte godkänna alls.
    pub fn approval_ceiling(&self) -> Option<Decimal> {
        match self {
            Role::SuperAdmin     => None, // hanteras av has_unlimited_approval
            Role::OrgAdmin       => None, // hanteras av has_unlimited_approval
            Role::CFO            => Some(Decimal::from(500_000)),
            Role::AccountManager => Some(Decimal::from(100_000)),
            Role::Bookkeeper     => Some(Decimal::from(50_000)),
            Role::Auditor        => None,
            Role::APIClient      => None,
            Role::ReadOnly       => None,
        }
    }

    /// Returnerar den uppsättning actions som rollen ger.
    pub fn granted_actions(&self) -> Vec<Action> {
        match self {
            Role::SuperAdmin => Action::all(),

            Role::OrgAdmin => vec![
                Action::ViewTransaction, Action::BookTransaction,
                Action::ReverseTransaction, Action::ExportTransactions,
                Action::ViewInvoice, Action::ApproveInvoice, Action::RejectInvoice,
                Action::CreateVoucher,
                Action::ViewPayment, Action::InitiatePayment, Action::ApprovePayment,
                Action::ViewVatReport, Action::ExportAuditPack,
                Action::ViewTreasuryDashboard,
                Action::ManageUsers, Action::ManageIntegrations, Action::ViewAuditLog,
                Action::ApproveUnder10k, Action::ApproveUnder100k, Action::ApproveUnlimited,
            ],

            Role::CFO => vec![
                Action::ViewTransaction, Action::BookTransaction,
                Action::ReverseTransaction, Action::ExportTransactions,
                Action::ViewInvoice, Action::ApproveInvoice, Action::RejectInvoice,
                Action::CreateVoucher,
                Action::ViewPayment, Action::InitiatePayment, Action::ApprovePayment,
                Action::ViewVatReport, Action::ExportAuditPack,
                Action::ViewTreasuryDashboard, Action::ViewAuditLog,
                Action::ApproveUnder10k, Action::ApproveUnder100k,
            ],

            Role::AccountManager => vec![
                Action::ViewTransaction, Action::BookTransaction,
                Action::ExportTransactions,
                Action::ViewInvoice, Action::ApproveInvoice, Action::RejectInvoice,
                Action::CreateVoucher,
                Action::ViewPayment, Action::InitiatePayment, Action::ApprovePayment,
                Action::ViewVatReport,
                Action::ApproveUnder10k, Action::ApproveUnder100k,
            ],

            Role::Bookkeeper => vec![
                Action::ViewTransaction, Action::BookTransaction,
                Action::ExportTransactions,
                Action::ViewInvoice, Action::CreateVoucher,
                Action::ViewPayment,
                Action::ViewVatReport,
                Action::ApproveUnder10k,
            ],

            Role::Auditor => vec![
                Action::ViewTransaction, Action::ExportTransactions,
                Action::ViewInvoice,
                Action::ViewPayment,
                Action::ViewVatReport, Action::ExportAuditPack,
                Action::ViewAuditLog,
            ],

            Role::APIClient => vec![
                Action::ViewTransaction, Action::ViewInvoice,
                Action::ViewPayment, Action::ViewVatReport,
            ],

            Role::ReadOnly => vec![
                Action::ViewTransaction, Action::ViewInvoice,
                Action::ViewPayment, Action::ViewVatReport,
            ],
        }
    }
}

// ─── Action ───────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Action {
    // Transactions
    ViewTransaction,
    BookTransaction,
    ReverseTransaction,
    ExportTransactions,
    // Invoices
    ViewInvoice,
    ApproveInvoice,
    RejectInvoice,
    CreateVoucher,
    // Payments
    ViewPayment,
    InitiatePayment,
    ApprovePayment,
    // Reports
    ViewVatReport,
    ExportAuditPack,
    ViewTreasuryDashboard,
    // Admin
    ManageUsers,
    ManageIntegrations,
    ViewAuditLog,
    // Amount tiers
    ApproveUnder10k,
    ApproveUnder100k,
    ApproveUnlimited,
}

impl Action {
    pub fn all() -> Vec<Action> {
        vec![
            Action::ViewTransaction, Action::BookTransaction,
            Action::ReverseTransaction, Action::ExportTransactions,
            Action::ViewInvoice, Action::ApproveInvoice, Action::RejectInvoice,
            Action::CreateVoucher,
            Action::ViewPayment, Action::InitiatePayment, Action::ApprovePayment,
            Action::ViewVatReport, Action::ExportAuditPack, Action::ViewTreasuryDashboard,
            Action::ManageUsers, Action::ManageIntegrations, Action::ViewAuditLog,
            Action::ApproveUnder10k, Action::ApproveUnder100k, Action::ApproveUnlimited,
        ]
    }

    /// Är denna action en write-action (muterande)?
    pub fn is_mutating(&self) -> bool {
        matches!(
            self,
            Action::BookTransaction
                | Action::ReverseTransaction
                | Action::ApproveInvoice
                | Action::RejectInvoice
                | Action::CreateVoucher
                | Action::InitiatePayment
                | Action::ApprovePayment
                | Action::ManageUsers
                | Action::ManageIntegrations
        )
    }

    /// Kräver denna action explicit belopps-behörighet?
    pub fn is_amount_gated(&self) -> bool {
        matches!(
            self,
            Action::ApproveInvoice
                | Action::ApprovePayment
                | Action::InitiatePayment
                | Action::BookTransaction
        )
    }
}

// ─── Resource ─────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct Resource {
    pub resource_type: ResourceType,
    /// Vilket bolag/entitet resursen tillhör.
    pub entity_id: Option<String>,
    /// Belopp i SEK (eller lokal valuta, konverterad).
    pub amount: Option<Decimal>,
}

impl Resource {
    pub fn new(resource_type: ResourceType) -> Self {
        Self { resource_type, entity_id: None, amount: None }
    }

    pub fn with_entity(mut self, entity_id: impl Into<String>) -> Self {
        self.entity_id = Some(entity_id.into());
        self
    }

    pub fn with_amount(mut self, amount: Decimal) -> Self {
        self.amount = Some(amount);
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResourceType {
    Transaction,
    Invoice,
    Payment,
    Report,
    AuditLog,
    Settings,
}

// ─── Context (ABAC) ───────────────────────────────────────────────────────────

#[derive(Debug, Clone, Default)]
pub struct Context {
    /// Är 2FA verifierad i denna session?
    pub two_factor_verified: bool,
    /// Finns en andra godkännare tillgänglig (för dual-approval)?
    pub second_approver_id: Option<String>,
    /// Tidsstämpel för begäran (används för t.ex. out-of-hours-regler).
    pub request_timestamp_utc: Option<i64>,
    /// Fritext-attribut för utökad ABAC-logik.
    pub attributes: HashMap<String, String>,
}

impl Context {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_2fa(mut self) -> Self {
        self.two_factor_verified = true;
        self
    }

    pub fn with_second_approver(mut self, approver_id: impl Into<String>) -> Self {
        self.second_approver_id = Some(approver_id.into());
        self
    }

    pub fn with_attribute(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.attributes.insert(key.into(), value.into());
        self
    }
}

// ─── PermissionResult ─────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct PermissionResult {
    pub allowed: bool,
    pub reason: String,
    pub requires_2fa: bool,
    pub requires_second_approver: bool,
}

impl PermissionResult {
    fn allow(reason: impl Into<String>) -> Self {
        Self {
            allowed: true,
            reason: reason.into(),
            requires_2fa: false,
            requires_second_approver: false,
        }
    }

    fn deny(reason: impl Into<String>) -> Self {
        Self {
            allowed: false,
            reason: reason.into(),
            requires_2fa: false,
            requires_second_approver: false,
        }
    }

    fn allow_with_2fa(reason: impl Into<String>) -> Self {
        Self {
            allowed: true,
            reason: reason.into(),
            requires_2fa: true,
            requires_second_approver: false,
        }
    }

    fn allow_with_dual_approval(reason: impl Into<String>) -> Self {
        Self {
            allowed: true,
            reason: reason.into(),
            requires_2fa: true,
            requires_second_approver: true,
        }
    }
}

// ─── PolicyStore ──────────────────────────────────────────────────────────────

/// Håller konfigurerbara policies. Hårda regler i PermissionEngine
/// kan inte åsidosättas härifrån.
#[derive(Debug, Clone)]
pub struct PolicyStore {
    /// Bokföringsgräns utan extra godkännande per roll.
    pub booking_limit_override: HashMap<Role, Decimal>,
    /// Extra entitets-specifika regler (entity_id → verb-blocklist).
    pub entity_restrictions: HashMap<String, Vec<Action>>,
}

impl PolicyStore {
    pub fn new() -> Self {
        let mut booking_limit_override = HashMap::new();
        // Standardgränser; kan skrivas över vid initiering.
        booking_limit_override.insert(Role::Bookkeeper,     Decimal::from(50_000));
        booking_limit_override.insert(Role::AccountManager, Decimal::from(100_000));
        booking_limit_override.insert(Role::CFO,            Decimal::from(500_000));
        Self {
            booking_limit_override,
            entity_restrictions: HashMap::new(),
        }
    }

    pub fn booking_limit_for(&self, role: &Role) -> Option<Decimal> {
        self.booking_limit_override.get(role).copied()
    }
}

impl Default for PolicyStore {
    fn default() -> Self { Self::new() }
}

// ─── PermissionEngine ─────────────────────────────────────────────────────────

pub struct PermissionEngine {
    policy_store: Arc<PolicyStore>,
}

impl PermissionEngine {
    pub fn new() -> Self {
        Self { policy_store: Arc::new(PolicyStore::new()) }
    }

    pub fn with_policy_store(store: PolicyStore) -> Self {
        Self { policy_store: Arc::new(store) }
    }

    // ── Public API ────────────────────────────────────────────────────────────

    /// Kontrollera om en användare får utföra en action utan extra kontext.
    pub fn can(
        &self,
        subject: &Subject,
        action: Action,
        resource: &Resource,
    ) -> PermissionResult {
        self.can_with_context(subject, action, resource, &Context::new())
    }

    /// Kontrollera med attribut-baserade regler (ABAC).
    pub fn can_with_context(
        &self,
        subject: &Subject,
        action: Action,
        resource: &Resource,
        ctx: &Context,
    ) -> PermissionResult {
        // 1. Hårda regler – kontrolleras alltid oavsett roller.
        if let Some(denial) = self.hard_deny(subject, &action, resource) {
            return denial;
        }

        // 2. Entitets-access – om resursen tillhör ett specifikt bolag,
        //    kontrollera att subjektet har access.
        if let Some(ref entity_id) = resource.entity_id {
            if !self.subject_has_entity_access(subject, entity_id) {
                return PermissionResult::deny(format!(
                    "Användaren saknar access till entitet '{entity_id}'"
                ));
            }
        }

        // 3. Entitets-specifika begränsningar från PolicyStore.
        if let Some(ref entity_id) = resource.entity_id {
            if let Some(blocked) = self.policy_store.entity_restrictions.get(entity_id) {
                if blocked.contains(&action) {
                    return PermissionResult::deny(format!(
                        "Action '{:?}' är blockerad för entitet '{entity_id}'",
                        action
                    ));
                }
            }
        }

        // 4. RBAC — rollen måste ha actions i sin grant-lista.
        if !self.role_permits(subject, &action) {
            return PermissionResult::deny(format!(
                "Ingen av användarens roller tillåter '{:?}'",
                action
            ));
        }

        // 5. ABAC — beloppsgränser och dual-approval.
        self.abac_amount_check(subject, &action, resource, ctx)
    }

    /// Lista alla tillåtna actions för ett subject mot en resurs.
    pub fn allowed_actions(
        &self,
        subject: &Subject,
        resource: &Resource,
    ) -> Vec<Action> {
        Action::all()
            .into_iter()
            .filter(|a| self.can(subject, a.clone(), resource).allowed)
            .collect()
    }

    // ── Internals ─────────────────────────────────────────────────────────────

    /// Hårda regler som aldrig kan åsidosättas av konfiguration.
    fn hard_deny(
        &self,
        subject: &Subject,
        action: &Action,
        _resource: &Resource,
    ) -> Option<PermissionResult> {
        // REGEL 1: Auditor kan ALDRIG ändra data.
        let is_auditor_only = subject.roles.iter().all(|r| matches!(r, Role::Auditor | Role::ReadOnly | Role::APIClient))
            && subject.roles.iter().any(|r| matches!(r, Role::Auditor));
        if is_auditor_only && action.is_mutating() {
            return Some(PermissionResult::deny(
                "Auditor-rollen tillåter ej muterande actions",
            ));
        }

        // REGEL 2: ReadOnly kan aldrig utföra muterande actions.
        if subject.roles.iter().all(|r| matches!(r, Role::ReadOnly))
            && action.is_mutating()
        {
            return Some(PermissionResult::deny(
                "ReadOnly-rollen tillåter ej muterande actions",
            ));
        }

        // REGEL 3: Reversal kräver CFO eller OrgAdmin.
        if *action == Action::ReverseTransaction {
            let can_reverse = subject.has_any_role(&[Role::CFO, Role::OrgAdmin, Role::SuperAdmin]);
            if !can_reverse {
                return Some(PermissionResult::deny(
                    "Reversal kräver CFO eller OrgAdmin",
                ));
            }
        }

        // REGEL 4: Betalningar >100 000 SEK kräver alltid two-party approval.
        //          Detta är en informerande hård regel — vi sätter requires_second_approver.
        //          Faktisk spärr sker i abac_amount_check.

        None
    }

    /// Kontrollerar att minst en roll hos subject tillåter actionen via RBAC.
    fn role_permits(&self, subject: &Subject, action: &Action) -> bool {
        subject.roles.iter().any(|role| role.granted_actions().contains(action))
    }

    /// Kontrollerar beloppsbaserade regler.
    fn abac_amount_check(
        &self,
        subject: &Subject,
        action: &Action,
        resource: &Resource,
        ctx: &Context,
    ) -> PermissionResult {
        let amount = match resource.amount {
            Some(a) if action.is_amount_gated() => a,
            _ => return PermissionResult::allow("Beviljat via RBAC"),
        };

        // Dual-approval: betalning eller godkännande >100 000 SEK.
        let dual_approval_threshold = Decimal::from(100_000);
        if amount > dual_approval_threshold
            && matches!(action, Action::ApprovePayment | Action::InitiatePayment | Action::ApproveInvoice)
        {
            if ctx.second_approver_id.is_none() {
                return PermissionResult {
                    allowed: false,
                    reason: format!(
                        "Belopp {amount} SEK > 100 000 SEK kräver two-party approval (saknar second_approver_id)"
                    ),
                    requires_2fa: true,
                    requires_second_approver: true,
                };
            }
            // Även om andra godkännaren finns: kräv 2FA.
            if !ctx.two_factor_verified {
                return PermissionResult {
                    allowed: false,
                    reason: "Belopp >100 000 SEK kräver verifierad 2FA".into(),
                    requires_2fa: true,
                    requires_second_approver: true,
                };
            }
            // Kontrollera att subject har tillräcklig roll för beloppet.
            return self.check_amount_role_limit(subject, amount, action, true);
        }

        self.check_amount_role_limit(subject, amount, action, false)
    }

    fn check_amount_role_limit(
        &self,
        subject: &Subject,
        amount: Decimal,
        action: &Action,
        dual_approval_present: bool,
    ) -> PermissionResult {
        // SuperAdmin/OrgAdmin → alltid ok.
        if subject.has_any_role(&[Role::SuperAdmin, Role::OrgAdmin]) {
            return if dual_approval_present {
                PermissionResult::allow_with_dual_approval("OrgAdmin/SuperAdmin med dual approval")
            } else {
                PermissionResult::allow("OrgAdmin/SuperAdmin")
            };
        }

        // CFO: upp till 500 000 SEK.
        if subject.has_any_role(&[Role::CFO]) {
            let limit = self.policy_store.booking_limit_for(&Role::CFO)
                .unwrap_or(Decimal::from(500_000));
            if amount <= limit {
                return if dual_approval_present {
                    PermissionResult::allow_with_dual_approval(
                        format!("CFO godkänner {amount} SEK (gräns {limit} SEK) med dual approval")
                    )
                } else {
                    PermissionResult::allow(
                        format!("CFO godkänner {amount} SEK (gräns {limit} SEK)")
                    )
                };
            }
            return PermissionResult::deny(
                format!("CFO-gräns är {limit} SEK, begärt {amount} SEK")
            );
        }

        // AccountManager: upp till 100 000 SEK.
        if subject.has_any_role(&[Role::AccountManager]) {
            let limit = self.policy_store.booking_limit_for(&Role::AccountManager)
                .unwrap_or(Decimal::from(100_000));
            if amount <= limit {
                return PermissionResult::allow(
                    format!("AccountManager godkänner {amount} SEK (gräns {limit} SEK)")
                );
            }
            return PermissionResult::deny(
                format!("AccountManager-gräns är {limit} SEK, begärt {amount} SEK")
            );
        }

        // Bookkeeper: upp till 50 000 SEK, och bara för BookTransaction.
        if subject.has_any_role(&[Role::Bookkeeper]) {
            if !matches!(action, Action::BookTransaction | Action::CreateVoucher) {
                return PermissionResult::deny(
                    "Bookkeeper kan inte godkänna betalningar eller fakturor"
                );
            }
            let limit = self.policy_store.booking_limit_for(&Role::Bookkeeper)
                .unwrap_or(Decimal::from(50_000));
            if amount <= limit {
                return PermissionResult::allow(
                    format!("Bookkeeper bokför {amount} SEK (gräns {limit} SEK)")
                );
            }
            return PermissionResult::deny(
                format!("Bookkeeper-gräns är {limit} SEK, begärt {amount} SEK — kräver AccountManager eller högre")
            );
        }

        PermissionResult::deny("Ingen roll med tillräcklig beloppsrättighet hittades")
    }

    /// Kontrollerar om subject har access till angiven entitet.
    fn subject_has_entity_access(&self, subject: &Subject, entity_id: &str) -> bool {
        // SuperAdmin och OrgAdmin har alltid full koncernaccess.
        if subject.has_any_role(&[Role::SuperAdmin, Role::OrgAdmin]) {
            return true;
        }
        // Övriga roller: explicit lista.
        subject.entity_access.iter().any(|e| e == entity_id)
    }
}

impl Default for PermissionEngine {
    fn default() -> Self { Self::new() }
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn engine() -> PermissionEngine { PermissionEngine::new() }

    fn subject(role: Role) -> Subject {
        Subject::new("user-1", "org-1")
            .with_roles(vec![role])
            .with_entity_access(vec!["entity-1".into()])
    }

    fn txn_resource(amount: Option<u64>, entity: Option<&str>) -> Resource {
        let mut r = Resource::new(ResourceType::Transaction);
        if let Some(a) = amount { r = r.with_amount(Decimal::from(a)); }
        if let Some(e) = entity { r = r.with_entity(e); }
        r
    }

    // ── Auditor hard-deny ──────────────────────────────────────────────────────

    #[test]
    fn auditor_cannot_book_transaction() {
        let e = engine();
        let s = subject(Role::Auditor);
        let r = txn_resource(None, None);
        let result = e.can(&s, Action::BookTransaction, &r);
        assert!(!result.allowed, "Auditor ska ALDRIG kunna bokföra");
        assert!(result.reason.contains("Auditor"));
    }

    #[test]
    fn auditor_can_view_and_export() {
        let e = engine();
        let s = subject(Role::Auditor);
        let r = Resource::new(ResourceType::AuditLog);
        assert!(e.can(&s, Action::ViewTransaction, &r).allowed);
        assert!(e.can(&s, Action::ExportAuditPack, &r).allowed);
    }

    #[test]
    fn auditor_cannot_reverse_transaction() {
        let e = engine();
        let s = subject(Role::Auditor);
        let r = txn_resource(None, None);
        assert!(!e.can(&s, Action::ReverseTransaction, &r).allowed);
    }

    // ── Reversal requires CFO/OrgAdmin ────────────────────────────────────────

    #[test]
    fn bookkeeper_cannot_reverse() {
        let e = engine();
        let s = subject(Role::Bookkeeper);
        let r = txn_resource(None, None);
        let result = e.can(&s, Action::ReverseTransaction, &r);
        assert!(!result.allowed);
        assert!(result.reason.contains("Reversal"));
    }

    #[test]
    fn cfo_can_reverse() {
        let e = engine();
        let s = subject(Role::CFO);
        let r = txn_resource(None, None);
        assert!(e.can(&s, Action::ReverseTransaction, &r).allowed);
    }

    #[test]
    fn org_admin_can_reverse() {
        let e = engine();
        let s = subject(Role::OrgAdmin);
        let r = txn_resource(None, None);
        assert!(e.can(&s, Action::ReverseTransaction, &r).allowed);
    }

    // ── Bookkeeper booking limit ───────────────────────────────────────────────

    #[test]
    fn bookkeeper_can_book_under_50k() {
        let e = engine();
        let s = subject(Role::Bookkeeper);
        let r = txn_resource(Some(49_999), None);
        assert!(e.can(&s, Action::BookTransaction, &r).allowed);
    }

    #[test]
    fn bookkeeper_cannot_book_over_50k() {
        let e = engine();
        let s = subject(Role::Bookkeeper);
        let r = txn_resource(Some(50_001), None);
        let result = e.can(&s, Action::BookTransaction, &r);
        assert!(!result.allowed);
        assert!(
            result.reason.contains("50000") || result.reason.contains("50 000"),
            "Unexpected reason: {}", result.reason
        );
    }

    // ── AccountManager limit ──────────────────────────────────────────────────

    #[test]
    fn account_manager_can_approve_under_100k() {
        let e = engine();
        let s = subject(Role::AccountManager);
        let r = Resource::new(ResourceType::Invoice)
            .with_amount(Decimal::from(99_999));
        assert!(e.can(&s, Action::ApproveInvoice, &r).allowed);
    }

    #[test]
    fn account_manager_blocked_over_100k() {
        let e = engine();
        let s = subject(Role::AccountManager);
        let r = Resource::new(ResourceType::Invoice)
            .with_amount(Decimal::from(100_001));
        let result = e.can(&s, Action::ApproveInvoice, &r);
        assert!(!result.allowed);
    }

    // ── CFO limit ─────────────────────────────────────────────────────────────

    #[test]
    fn cfo_can_approve_up_to_500k() {
        let e = engine();
        let s = subject(Role::CFO);
        let r = Resource::new(ResourceType::Payment)
            .with_amount(Decimal::from(499_999));
        let ctx = Context::new().with_2fa().with_second_approver("approver-2");
        assert!(e.can_with_context(&s, Action::ApprovePayment, &r, &ctx).allowed);
    }

    #[test]
    fn cfo_blocked_over_500k() {
        let e = engine();
        let s = subject(Role::CFO);
        let r = Resource::new(ResourceType::Payment)
            .with_amount(Decimal::from(500_001));
        let ctx = Context::new().with_2fa().with_second_approver("approver-2");
        assert!(!e.can_with_context(&s, Action::ApprovePayment, &r, &ctx).allowed);
    }

    // ── Dual-approval >100k ───────────────────────────────────────────────────

    #[test]
    fn payment_over_100k_requires_second_approver() {
        let e = engine();
        let s = subject(Role::CFO);
        let r = Resource::new(ResourceType::Payment)
            .with_amount(Decimal::from(150_000));
        let ctx_no_second = Context::new().with_2fa();
        let result = e.can_with_context(&s, Action::ApprovePayment, &r, &ctx_no_second);
        assert!(!result.allowed);
        assert!(result.requires_second_approver);
    }

    #[test]
    fn payment_over_100k_allowed_with_dual_approval() {
        let e = engine();
        let s = subject(Role::CFO);
        let r = Resource::new(ResourceType::Payment)
            .with_amount(Decimal::from(150_000));
        let ctx = Context::new().with_2fa().with_second_approver("approver-2");
        assert!(e.can_with_context(&s, Action::ApprovePayment, &r, &ctx).allowed);
    }

    // ── Entity access ─────────────────────────────────────────────────────────

    #[test]
    fn bookkeeper_blocked_from_unauthorized_entity() {
        let e = engine();
        let s = Subject::new("u1", "org-1")
            .with_roles(vec![Role::Bookkeeper])
            .with_entity_access(vec!["entity-A".into()]);
        let r = txn_resource(Some(100), Some("entity-B"));
        assert!(!e.can(&s, Action::ViewTransaction, &r).allowed);
    }

    #[test]
    fn org_admin_has_all_entity_access() {
        let e = engine();
        let s = Subject::new("u1", "org-1")
            .with_roles(vec![Role::OrgAdmin]);
        // OrgAdmin har inga entiteter explicit men ska ändå ha access.
        let r = txn_resource(Some(100), Some("any-entity-42"));
        assert!(e.can(&s, Action::ViewTransaction, &r).allowed);
    }

    // ── allowed_actions ───────────────────────────────────────────────────────

    #[test]
    fn auditor_allowed_actions_are_read_only() {
        let e = engine();
        let s = subject(Role::Auditor);
        let r = Resource::new(ResourceType::Transaction);
        let actions = e.allowed_actions(&s, &r);
        for a in &actions {
            assert!(!a.is_mutating(), "Auditor ska inte ha muterande action {:?}", a);
        }
        assert!(!actions.is_empty());
    }
}
