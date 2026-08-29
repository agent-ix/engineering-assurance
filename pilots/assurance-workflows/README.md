# Opt-in workflow definitions

These definitions coordinate existing authoring, validation, evidence, and
review operations. They are not installed by default and do not replace the
decision owner.

Start with:

```bash
ix-flow run <name> --path pilots/assurance-workflows --id <run-id> --json
```

Follow the returned actions. Do not select, acknowledge, or retry a terminal
transition until the named decision owner has chosen its exact outcome.
