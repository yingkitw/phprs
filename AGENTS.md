# Agent Development Loop

This document defines the continuous improvement cycle for the project.

## The Loop

### 1. Complete Remaining TODO Items
Pick the next highest-priority item from `TODO.md` (or `ARCHITECTURE.md` if the task is architectural). Implement it with minimal, focused changes. Do not add speculative features.

### 2. Create Tests and Examples
For every new capability:
- Write integration tests in `tests/` and unit tests next to components that exercise the feature end-to-end where possible
- Add unit tests for core logic where appropriate
- Provide a minimal usage example if the feature is client-facing

### 3. Ensure `cargo test` Passes
Run the full test suite. Fix any failures before proceeding. Warnings are acceptable but should be noted.

### 4. Loop Back to Step 1
Return to `TODO.md` and pick the next item. Repeat until the backlog is clear.

### 5. Audit and Optimize
After each batch of features, perform a quality pass:
- **Maintainability**: Are functions small and well-named? Is the module structure logical?
- **Leanness**: Remove dead code, unused imports, and speculative abstractions
- **Wiring**: Ensure all new features are properly integrated into `main.rs`, `Cargo.toml` scripts, and docs
- **Small footprint**: Avoid unnecessary crates; prefer the standard library or lightweight dependencies
- **Consistency**: Match existing code style and patterns

### 6. Competitive Intelligence
Research similar open-source TeX to PDF converters (Tectonic, Pandoc, Typst, texlive). Identify capabilities they have that this project lacks. Add the most valuable ones to the `TODO.md` brainstorming section. Prioritize features that provide clear competitive advantage.

### 7. Update Documentation
Keep all project docs aligned with the current implementation:
- **`README.md`**: Quick start, feature list, architecture summary
- **`TODO.md`**: Mark completed items, move them to Done, keep brainstorming current
- **`SPEC.md`**: Scope and requirements for the site, technical stack, quality bar
- **`ARCHITECTURE.md`**: Module relationships, data flow, deployment topology
- **`AGENTS.md`**: This file — update if the loop itself evolves

## Principles

- **Simplicity over flexibility**: Solve the problem at hand, not every hypothetical future problem
- **Surgical changes**: Touch only what you must; clean up only your own mess
- **Goal-driven**: Every change should have a verifiable success criterion
- **Test before ship**: No feature is complete until it has passing tests
- **Docs are code**: Documentation drift is a bug
