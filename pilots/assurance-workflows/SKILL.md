---
name: assurance-workflows
description: Coordinate four opt-in engineering decision workflows without automating terminal choices.
contributes:
  workflows: ./workflows
---

# Assurance workflows

Run a workflow by name with `ix-flow` in path mode. Record only validated
artifacts and producer outputs. Terminal transitions always require a named
human decision; automation must not acknowledge or override those gates.

Supported names are `assurance-intake`, `architecture-evaluation`,
`measurement-promotion`, and `change-assurance`.
