"""Construct metadata of the generated types (FR-079, FR-142).

Rendered from the `x-agent-ix-*` construct annotations of the JSON Schema
documents this package was generated from. Do not edit: regenerate through
the backend seam."""

from __future__ import annotations

#: The construct kind of each generated type.
TYPE_KIND: dict[str, str] = {
    'CampaignAttemptStatus': 'campaign_enum',
    'CampaignCompletionRule': 'campaign_enum',
    'CampaignVerdict': 'campaign_enum',
    'ProcedureArgumentKind': 'campaign_enum',
    'ProcedureEnvironmentKind': 'campaign_enum',
    'ProcedureInputOriginKind': 'campaign_enum',
}

#: The types each type specializes; its class carries their fields.
SUPERTYPES: dict[str, tuple[str, ...]] = {}

#: The types that have no direct instances.
ABSTRACT: dict[str, bool] = {}

#: The fields that tell a type's instances apart, in declared order.
IDENTITY_FIELDS: dict[str, tuple[str, ...]] = {}

#: The owner whose instance a nested entity's instance exists within.
OWNER: dict[str, str] = {}

#: The member types of an aggregate root's boundary or a domain's namespace.
MEMBERS: dict[str, tuple[str, ...]] = {}

#: The field recording the instant an event occurred.
OCCURRENCE_FIELD: dict[str, str] = {}

#: The value objects: two instances are equal when every field is equal.
VALUE_EQUALITY: dict[str, bool] = {}

#: The events: an instance records one occurrence and does not change.
IMMUTABLE: dict[str, bool] = {}

#: Each transition as (from, to, trigger operation, guard clause id, emitted events).
TRANSITIONS: dict[str, tuple[tuple[str, str, str, str | None, tuple[str, ...]], ...]] = {}

#: A process's ordered steps as (name, step kind, consumed events, emitted events).
STEPS: dict[str, tuple[tuple[str, str, tuple[str, ...], tuple[str, ...]], ...]] = {}

#: The types a repository persists.
PERSISTS: dict[str, tuple[str, ...]] = {}

#: A domain's vocabulary as (term, doc).
VOCABULARY: dict[str, tuple[tuple[str, str], ...]] = {}

#: The inherited fields whose values include each field's values.
FIELD_SUBSETS: dict[str, dict[str, tuple[str, ...]]] = {}

#: The inherited field each field narrows in its place.
FIELD_REDEFINES: dict[str, dict[str, str]] = {}

#: The feature paths each operation modifies, creates and deletes.
OPERATION_FRAMES: dict[str, dict[str, tuple[str, ...]]] = {}

#: Each operation's inline pre- and postconditions as (language, text).
OPERATION_CLAUSES: dict[str, dict[str, tuple[tuple[str, str], ...]]] = {}

#: Each named population's extent (closed or open) and its member types.
POPULATIONS: dict[str, tuple[str, tuple[str, ...]]] = {}
