# Basic model default

status: implemented

## Goal

Keep the basic model implementation contract-clean. A real provider, mock, deterministic echo, or other placeholder is acceptable if it implements the same canonical model contract and does not create a second model API.

## Required changes

1. Make `phenix-plugin-basic-model` implement the canonical model interface used by replaceable model providers.
2. Keep provider selection, credentials, authority, model identity, and routing in their canonical owners. Do not duplicate them inside the basic model implementation.
3. Preserve replacement. Consumers bind through the neutral model interface and do not depend on the basic implementation crate.
4. Preserve omission. Removing the basic model from a composition does not create a hidden Core fallback.
5. Remove any echo-specific, mock-specific, legacy, or compatibility contract path. Placeholder behavior must flow through the normal model contract.
6. Keep placeholder behavior explicit in implementation identity or configuration where ambiguity would mislead a caller. Do not pretend deterministic fixture behavior is external model inference.

## Placeholder rule

Mocks and placeholders are valid first-party implementations.

A placeholder is clean when:

- it implements the same request, response, streaming, failure, and authority contract expected of any model provider;
- replacing it with a real provider requires composition or configuration changes, not consumer source changes;
- no special consumer API exists for the placeholder;
- no compatibility shim preserves an older model contract;
- tests may use deterministic behavior without changing production contract ownership.

Deterministic echo behavior may remain if it satisfies these rules. It does not need to move to test support solely because it is a mock.

## Validation

Add regressions that prove:

- the basic model uses the canonical model contract;
- a deterministic placeholder and a real provider can satisfy the same consumer contract;
- replacing the basic model requires no consumer source change;
- omitting it produces the normal missing-provider behavior rather than hidden fallback behavior;
- no mock-only or legacy model contract is exported.

The final exact head must pass Source, Rust, Product, and Maintenance validation.

## Completion

This slice is complete when `phenix.basic-model` is an ordinary replaceable implementation of the canonical model contract, regardless of whether its current behavior is real inference or an explicit placeholder.
