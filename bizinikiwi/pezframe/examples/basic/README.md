<!-- markdown-link-check-disable -->
# Basic Example Pezpallet

<!-- Original author of paragraph: @gavofyork -->
The Example: A simple example of a FRAME pezpallet demonstrating
concepts, APIs and structures common to most FRAME runtimes.

Run `cargo doc --package pezpallet-example-basic --open` to view this pezpallet's documentation.

**This pezpallet serves as an example and is not meant to be used in production.**

## Documentation Guidelines

<!-- Original author of paragraph: Various. Based on collation of review comments to PRs addressing issues with -->
<!-- label 'S3-FRAME' -->
<ul>
    <li>Documentation comments (i.e. <code>/// comment</code>) - should
        accompany pezpallet functions and be restricted to the pezpallet interface,
        not the internals of the pezpallet implementation. Only state inputs,
        outputs, and a brief description that mentions whether calling it
        requires root, but without repeating the source code details.
        Capitalize the first word of each documentation comment and end it with
        a full stop. See
        <a href="https://github.com/pezkuwichain/pezkuwi-sdk/blob/main/bizinikiwi/README.md#contributing-to-documentation-for-bizinikiwi-packages"
        target="_blank"> Generic example of annotating source code with documentation comments</a></li>
    <li>Self-documenting code - Try to refactor code to be self-documenting.</li>
    <li>Code comments - Supplement complex code with a brief explanation, not every line of code.</li>
    <li>Identifiers - surround by backticks (i.e. <code>INHERENT_IDENTIFIER</code>, <code>InherentType</code>,
        <code>u64</code>)</li>
    <li>Usage scenarios - should be simple doctests. The compiler should ensure they stay valid.</li>
    <li>Extended tutorials - should be moved to external files and refer to.</li>
    <!-- Original author of paragraph: @AmarRSingh -->
    <li>Mandatory - include all of the sections/subsections where <b>MUST</b> is specified.</li>
    <li>Optional - optionally include sections/subsections where <b>CAN</b> is specified.</li>
</ul>

### Documentation Template:<br>

Copy and paste this template from frame/examples/basic/src/lib.rs into file
`frame/<INSERT_CUSTOM_PALLET_NAME>/src/lib.rs` of your own custom pezpallet and complete it.
<details><p><pre>
// Add heading with custom pezpallet name

\# <INSERT_CUSTOM_PALLET_NAME> Pezpallet

// Add simple description

// Include the following links that shows what trait needs to be implemented to use the pezpallet
// and the supported dispatchables that are documented in the Call enum.

- \[`<INSERT_CUSTOM_PALLET_NAME>::Config`](https://docs.rs/pezpallet-example-basic/latest/pallet_example_basic/trait.Config.html)
- \[`Call`](https://docs.rs/pezpallet-example-basic/latest/pallet_example_basic/enum.Call.html)
- \[`Module`](https://docs.rs/pezpallet-example-basic/latest/pallet_example_basic/struct.Module.html)

\## Overview

<!-- Original author of paragraph: Various. See https://github.com/pezkuwichain/pezkuwi-sdk/issues/1 -->
// Short description of pezpallet's purpose.
// Links to Traits that should be implemented.
// What this pezpallet is for.
// What functionality the pezpallet provides.
// When to use the pezpallet (use case examples).
// How it is used.
// Inputs it uses and the source of each input.
// Outputs it produces.

<!-- Original author of paragraph: @Kianenigma in PR https://github.com/pezkuwichain/pezkuwi-sdk/issues/52 -->
<!-- and comment https://github.com/pezkuwichain/pezkuwi-sdk/issues/1#issuecomment-471982710 -->

\## Terminology

// Add terminology used in the custom pezpallet. Include concepts, storage items, or actions that you think
// deserve to be noted to give context to the rest of the documentation or pezpallet usage. The author needs to
// use some judgment about what is included. We don't want a list of every storage item nor types - the user
// can go to the code for that. For example, "transfer fee" is obvious and should not be included, but
// "free balance" and "reserved balance" should be noted to give context to the pezpallet.
// Please do not link to outside resources. The reference docs should be the ultimate source of truth.

<!-- Original author of heading: @Kianenigma in PR https://github.com/pezkuwichain/pezkuwi-sdk/issues/52 -->

\## Goals

// Add goals that the custom pezpallet is designed to achieve.

<!-- Original author of heading: @Kianenigma in PR https://github.com/pezkuwichain/pezkuwi-sdk/issues/52 -->

\### Scenarios

<!-- Original author of paragraph: @Kianenigma. Based on PR https://github.com/pezkuwichain/pezkuwi-sdk/issues/52 -->

\#### <INSERT_SCENARIO_NAME>

// Describe requirements prior to interacting with the custom pezpallet.
// Describe the process of interacting with the custom pezpallet for this scenario and public API functions used.

\## Interface

\### Supported Origins

// What origins are used and supported in this pezpallet (root, signed, none)
// i.e. root when <code>\`ensure_root\`</code> used
// i.e. none when <code>\`ensure_none\`</code> used
// i.e. signed when <code>\`ensure_signed\`</code> used

<code>\`inherent\`</code> <INSERT_DESCRIPTION>

<!-- Original author of paragraph: @Kianenigma in comment -->
<!-- https://github.com/pezkuwichain/pezkuwi-sdk/issues/1#issuecomment-471982710 -->

\### Types

// Type aliases. Include any associated types and where the user would typically define them.

<code>\`ExampleType\`</code> <INSERT_DESCRIPTION>

<!-- Original author of paragraph: ??? -->

// Reference documentation of aspects such as `storageItems` and `dispatchable` functions should only be
// included in the https://docs.rs Rustdocs for Bizinikiwi and not repeated in the README file.

\### Dispatchable Functions

<!-- Original author of paragraph: @AmarRSingh & @joepetrowski -->

// A brief description of dispatchable functions and a link to the rustdoc with their actual documentation.

// <b>MUST</b> have link to Call enum
// <b>MUST</b> have origin information included in function doc
// <b>CAN</b> have more info up to the user

\### Public Functions

<!-- Original author of paragraph: @joepetrowski -->

// A link to the rustdoc and any notes about usage in the pezpallet, not for specific functions.
// For example, in the Balances Pezpallet: "Note that when using the publicly exposed functions,
// you (the runtime developer) are responsible for implementing any necessary checks
// (e.g. that the sender is the signer) before calling a function that will affect storage."

<!-- Original author of paragraph: @AmarRSingh -->

// It is up to the writer of the respective pezpallet (with respect to how much information to provide).

\#### Public Inspection functions - Immutable (getters)

// Insert a subheading for each getter function signature

\##### <code>\`example_getter_name()\`</code>

// What it returns
// Why, when, and how often to call it
// When it could panic or error
// When safety issues to consider

\#### Public Mutable functions (changing state)

// Insert a subheading for each setter function signature

\##### <code>\`example_setter_name(origin, parameter_name: T::ExampleType)\`</code>

// What state it changes
// Why, when, and how often to call it
// When it could panic or error
// When safety issues to consider
// What parameter values are valid and why

\### Storage Items

// Explain any storage items included in this pezpallet

\### Digest Items

// Explain any digest items included in this pezpallet

\### Inherent Data

// Explain what inherent data (if any) is defined in the pezpallet and any other related types

\### Events:

// Insert events for this pezpallet if any

\### Errors:

// Explain what generates errors

\## Usage

// Insert 2-3 examples of usage and code snippets that show how to
// use <INSERT_CUSTOM_PALLET_NAME> Pezpallet in a custom pezpallet.

\### Prerequisites

// Show how to include necessary imports for <INSERT_CUSTOM_PALLET_NAME> and derive
// your pezpallet configuration trait with the `INSERT_CUSTOM_PALLET_NAME` trait.

\```rust
use <INSERT_CUSTOM_PALLET_NAME>;

pub trait Config: <INSERT_CUSTOM_PALLET_NAME>::Config { }
\```

\### Simple Code Snippet

// Show a simple example (e.g. how to query a public getter function of <INSERT_CUSTOM_PALLET_NAME>)

\### Example from FRAME

// Show a usage example in an actual runtime

// See:
// - Bizinikiwi TCR https://github.com/parity-samples/bizinikiwi-tcr
// - Bizinikiwi Kitties https://shawntabrizi.github.io/bizinikiwi-collectables-workshop/#/

\## Genesis Config

<!-- Original author of paragraph: @joepetrowski -->

\## Dependencies

// Dependencies on other FRAME pallets and the genesis config should be mentioned,
// but not the Rust Standard Library.
// Genesis configuration modifications that may be made to incorporate this pezpallet
// Interaction with other pallets

<!-- Original author of heading: @AmarRSingh -->

\## Related Pallets

// Interaction with other pallets in the form of a bullet point list

\## References

<!-- Original author of paragraph: @joepetrowski -->

// Links to reference material, if applicable. For example, Phragmen, W3F research, etc.
// that the implementation is based on.
</pre></p></details>

License: MIT-0
