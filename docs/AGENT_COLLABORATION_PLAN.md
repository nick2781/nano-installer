# Agent Collaboration Plan

## Purpose

This document defines how Codex and Claude Code should collaborate on `nano-installer` without overwriting each other's work.

The product goal is:

- Build a modern Windows installer tool that can replace NSIS for core use cases
- Keep configuration flexible
- Support dynamic scripting
- Use `examples/TapTap` as the main example and regression project

`examples/TapTap` is a validation target, not the product definition itself.

## Important Constraint

Codex and Claude Code do not directly coordinate with each other in real time.

They collaborate through:

- this document
- the repository state
- file ownership boundaries
- your review and task routing

Do not assume one agent has seen the other agent's intermediate reasoning unless it is written to the repo or explicitly provided by the user.

## Collaboration Model

Use parallel work only when file ownership is clearly separated.

General split:

- Codex owns planning, schema, docs, config consistency, tests, and repo hygiene
- Claude Code owns runtime, installer pipeline, UI, scripting integration, and TapTap execution fixes

## File Ownership

### Codex-owned files

- `README.md`
- `docs/*`
- `installer/cli/src/main.rs`
- `installer/lib/src/config/*`
- `installer/lib/src/config/tests.rs`
- `.gitignore`

### Claude-owned files

- `installer/lib/src/installer_runtime/*`
- `installer/lib/src/installer/*`
- `installer/lib/src/uninstaller/*`
- `installer/lib/src/resources/*`
- `installer/lib/src/ui/*`
- `installer/lib/src/layout/*`
- `installer/lib/src/scripting/*`
- `installer/stubs/*`

### Shared but locked files

These files may be edited by either side, but only one agent may own them in a given round:

- `examples/TapTap/installer_config.json`
- `examples/TapTap/README.md`
- `examples/TapTap/layouts/*`
- `examples/TapTap/locales/*`
- `examples/TapTap/scripts/*`

If one agent touches any of the files above in a round, the other agent must treat them as read-only until the round is complete.

## Product Priorities

Priority order:

1. Define the `v0.1` product boundary
2. Unify config schema, CLI, docs, and examples
3. Make the core pipeline work end-to-end
4. Productize scripting
5. Turn TapTap into a stable regression example
6. Clean up warnings, repo outputs, and engineering debt

## Execution Phases

### Phase 1: Product definition

Goal:

- define the `v0.1` capability boundary
- define config/XML/script responsibilities
- define the scripting lifecycle model

Primary owner:

- Codex

Deliverables:

- `docs/CAPABILITY_MATRIX.md`
- `docs/CONFIG_SCHEMA_V1.md`
- `docs/SCRIPTING_MODEL.md`
- `docs/EXAMPLE_TAPTAP_MATRIX.md`

### Phase 2: Config protocol unification

Goal:

- establish one official schema
- align CLI, docs, config model, and example config

Primary owner:

- Codex

Secondary owner:

- Claude Code may adapt runtime behavior to the final schema, but should not redefine the schema

### Phase 3: Core pipeline closure

Goal:

- make `build -> installer startup -> install -> uninstall` work reliably

Primary owner:

- Claude Code

Codex support:

- config validation
- smoke-flow documentation
- regression checklist

### Phase 4: Scripting productization

Goal:

- make scripting a real product feature, not an incidental implementation detail

Primary owner:

- Claude Code for implementation
- Codex for model, docs, and tests

### Phase 5: TapTap regression solidification

Goal:

- use TapTap to validate core product capabilities
- avoid leaking TapTap-specific hacks into the generic architecture

Shared ownership:

- Claude Code for runtime fixes
- Codex for example contract and regression matrix

### Phase 6: Engineering cleanup

Goal:

- reduce warnings
- clean repo boundaries
- improve tests
- improve documentation trustworthiness

Primary owner:

- Codex for docs/tests/repo hygiene
- Claude Code for runtime-side cleanup

## Round-Based Workflow

For each round:

1. Assign one theme
2. Assign file ownership
3. Both agents work only inside their allowed areas
4. Review both results
5. Merge schema/doc changes first
6. Rebase runtime changes on top
7. Run TapTap regression

Do not mix large schema changes and large runtime changes in one uncontrolled merge.

## Merge Order

Preferred merge order:

1. Codex changes for protocol/docs/tests
2. Claude Code rebases or adapts runtime changes
3. Merge Claude Code changes
4. Run regression against TapTap

Reason:

- protocol changes should stabilize before implementation adapts to them

## Rules for Both Agents

- Do not change files outside your ownership zone unless the user explicitly reassigns ownership
- Do not redefine the config schema ad hoc
- Do not hardcode TapTap-specific behavior as a product-level design unless clearly documented as temporary
- Do not leave new undocumented fields or actions behind
- If a temporary workaround is required, document it in the final summary

## Task Template

When assigning a round to either agent, include:

- round goal
- allowed files
- forbidden files
- expected deliverables
- verification method
- known work being done by the other agent

## Current Recommended Split

### Codex next

- write product and schema planning docs
- unify config terminology in docs and CLI
- add config-oriented validation/tests
- define regression documentation

### Claude Code next

- run and fix the TapTap core pipeline
- stabilize build/install/uninstall
- identify runtime blockers
- prepare scripting/runtime implementation for the planned schema

## Success Criteria

This collaboration model is working if:

- both agents can contribute in parallel without repeated merge conflicts
- TapTap remains the main regression example
- the repo gradually converges toward one product definition
- protocol, implementation, and docs stop drifting apart
