-- =============================================================================
-- V002__audit_immutable.sql
-- Immutability triggers — audit_events, ledger_entries (posted), journals (posted)
-- =============================================================================

-- ---------------------------------------------------------------------------
-- 1. audit_events — append-only: block ALL UPDATE and DELETE
-- ---------------------------------------------------------------------------

CREATE OR REPLACE FUNCTION audit_events_immutable()
RETURNS TRIGGER LANGUAGE plpgsql AS $$
BEGIN
    IF TG_OP = 'DELETE' THEN
        RAISE EXCEPTION
            'audit_events is append-only: DELETE on row % is not permitted. Actor: %, Action: %',
            OLD.id, OLD.actor, OLD.action
            USING ERRCODE = 'integrity_constraint_violation';
    ELSIF TG_OP = 'UPDATE' THEN
        RAISE EXCEPTION
            'audit_events is append-only: UPDATE on row % is not permitted.',
            OLD.id
            USING ERRCODE = 'integrity_constraint_violation';
    END IF;
    RETURN NULL;
END;
$$;

CREATE TRIGGER audit_events_no_update
    BEFORE UPDATE ON audit_events
    FOR EACH ROW EXECUTE FUNCTION audit_events_immutable();

CREATE TRIGGER audit_events_no_delete
    BEFORE DELETE ON audit_events
    FOR EACH ROW EXECUTE FUNCTION audit_events_immutable();

COMMENT ON FUNCTION audit_events_immutable() IS
    'Prevents any mutation of audit_events rows — enforces append-only invariant';

-- ---------------------------------------------------------------------------
-- 2. ledger_entries — block UPDATE/DELETE when the parent journal is posted
-- ---------------------------------------------------------------------------

CREATE OR REPLACE FUNCTION ledger_entries_posted_immutable()
RETURNS TRIGGER LANGUAGE plpgsql AS $$
DECLARE
    v_journal_status journal_status;
BEGIN
    -- On DELETE we use OLD, on UPDATE we use OLD as well (NEW not yet committed)
    SELECT status INTO v_journal_status
    FROM journals
    WHERE id = OLD.journal_id;

    IF v_journal_status = 'posted' THEN
        RAISE EXCEPTION
            'Ledger entry % belongs to posted journal % and cannot be %d.',
            OLD.id, OLD.journal_id, TG_OP
            USING ERRCODE = 'integrity_constraint_violation';
    END IF;

    IF TG_OP = 'DELETE' THEN
        RETURN OLD;
    END IF;
    RETURN NEW;
END;
$$;

CREATE TRIGGER ledger_entries_no_update_when_posted
    BEFORE UPDATE ON ledger_entries
    FOR EACH ROW EXECUTE FUNCTION ledger_entries_posted_immutable();

CREATE TRIGGER ledger_entries_no_delete_when_posted
    BEFORE DELETE ON ledger_entries
    FOR EACH ROW EXECUTE FUNCTION ledger_entries_posted_immutable();

COMMENT ON FUNCTION ledger_entries_posted_immutable() IS
    'Prevents mutation of ledger_entries whose parent journal has status=posted';

-- ---------------------------------------------------------------------------
-- 3. posted_journals_immutable — block UPDATE of core fields once posted
--    Fields that may never change after posting:
--      date, description, posted_by, is_reversed
--    Fields that are still allowed to change:
--      reversed_by (set when creating a reversing entry), status (for reversal only)
-- ---------------------------------------------------------------------------

CREATE OR REPLACE FUNCTION posted_journals_immutable()
RETURNS TRIGGER LANGUAGE plpgsql AS $$
BEGIN
    -- Only enforce once the journal reaches 'posted' state
    IF OLD.status = 'posted' THEN

        -- Allow only the reversal workflow: status → reversed AND reversed_by being set
        IF NEW.status = 'reversed' AND NEW.reversed_by IS NOT NULL THEN
            -- Permit this specific state transition — nothing else may change
            IF NEW.date        IS DISTINCT FROM OLD.date        OR
               NEW.description IS DISTINCT FROM OLD.description OR
               NEW.posted_by   IS DISTINCT FROM OLD.posted_by   OR
               NEW.confidence  IS DISTINCT FROM OLD.confidence
            THEN
                RAISE EXCEPTION
                    'Journal % is posted: only the reversal fields (status, reversed_by) may be updated.',
                    OLD.id
                    USING ERRCODE = 'integrity_constraint_violation';
            END IF;
            RETURN NEW;
        END IF;

        -- Any other update to a posted journal is forbidden
        RAISE EXCEPTION
            'Journal % is posted and immutable. Attempted operation: % → %.',
            OLD.id, OLD.status, NEW.status
            USING ERRCODE = 'integrity_constraint_violation';
    END IF;

    RETURN NEW;
END;
$$;

CREATE TRIGGER posted_journals_immutable
    BEFORE UPDATE ON journals
    FOR EACH ROW EXECUTE FUNCTION posted_journals_immutable();

-- Prevent hard-deletion of posted journals entirely
CREATE OR REPLACE FUNCTION journals_no_delete_when_posted()
RETURNS TRIGGER LANGUAGE plpgsql AS $$
BEGIN
    IF OLD.status IN ('posted', 'reversed') THEN
        RAISE EXCEPTION
            'Journal % has status % and cannot be deleted.',
            OLD.id, OLD.status
            USING ERRCODE = 'integrity_constraint_violation';
    END IF;
    RETURN OLD;
END;
$$;

CREATE TRIGGER journals_no_delete_posted
    BEFORE DELETE ON journals
    FOR EACH ROW EXECUTE FUNCTION journals_no_delete_when_posted();

COMMENT ON FUNCTION posted_journals_immutable()         IS
    'Locks posted journals: only the reversal workflow (status→reversed + reversed_by) is allowed';
COMMENT ON FUNCTION journals_no_delete_when_posted()    IS
    'Prevents hard deletion of posted or reversed journals';

-- ---------------------------------------------------------------------------
-- 4. Helper function: emit_audit_event
--    Convenience wrapper called from application code or other triggers
--    to insert into audit_events without needing to know column order.
-- ---------------------------------------------------------------------------

CREATE OR REPLACE FUNCTION emit_audit_event(
    p_actor       TEXT,
    p_action      TEXT,
    p_entity_id   UUID,
    p_entity_type entity_type,
    p_payload     JSONB       DEFAULT NULL,
    p_reason      TEXT        DEFAULT NULL,
    p_source      TEXT        DEFAULT NULL,
    p_confidence  NUMERIC     DEFAULT NULL
)
RETURNS UUID LANGUAGE plpgsql AS $$
DECLARE
    v_id UUID;
BEGIN
    INSERT INTO audit_events (actor, action, entity_id, entity_type,
                               payload, reason, source, confidence)
    VALUES (p_actor, p_action, p_entity_id, p_entity_type,
            p_payload, p_reason, p_source, p_confidence)
    RETURNING id INTO v_id;

    RETURN v_id;
END;
$$;

COMMENT ON FUNCTION emit_audit_event IS
    'Convenience wrapper for INSERT INTO audit_events — use instead of raw INSERT';
