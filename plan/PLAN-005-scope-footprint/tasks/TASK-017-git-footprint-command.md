---
id: TASK-017
title: "Git footprint adapter and scope-footprint command"
type: Task
status: done
relationships:
  - target: "ix://agent-ix/engineering-assurance/PLAN-005"
    type: part_of
  - target: "ix://agent-ix/engineering-assurance/TASK-016"
    type: depends_on
  - target: "ix://agent-ix/engineering-assurance/FR-032"
    type: references
---

# TASK-017: Git footprint adapter and scope-footprint command

## Objective

Compute the footprint with git and expose the verdict as a CLI command.

## Deliverables

- `scope_footprint_host` and the `scope-footprint` subcommand.
- TC-237 and TC-238.

## Acceptance

FR-032-AC-6 and FR-032-AC-7 pass.
