 ## `#[bitmask]`

 Declares a *bitmask definition* using an `enum` and
 generates a corresponding transparent *bits type* for runtime use.

 This macro is intentionally **low-level**. It does **not** attempt to model
 permissions, states, or invariants. It only provides:

 - Explicit bit values
 - Compile-time composition of bits
 - A thin, transparent wrapper around the raw integer

 ### Overview

 Given an enum definition:

 ```rust
 #[bitmask(enable_auto_assign)]
 #[repr(u8)]
 pub enum Permissions {
     Read,
     Write,
     #[compound(Read | Write)]
     ReadWrite,
 }
 ```

 This macro:

 1. Resolves all enum discriminants into concrete bit expressions
 2. Expands `#[compound(...)]` expressions at compile time
 3. Generates a transparent `PermissionsBits` wrapper around the underlying
    integer representation
 4. Implements standard bitwise operators between:
    - enum variants
    - the generated bits type
    - the raw integer representation

 ### Representation

 A concrete integer representation is **required**:

 ```rust
 #[repr(u8 | u16 | u32 | u64 | u128 | usize)]
 ```

 Signed integer representations are **not supported**.
 Bitmasks are defined in terms of unsigned bitwise operations only.

 ### Variant Assignment Rules

 Each enum variant must satisfy **exactly one** of the following:

 - Have an explicit discriminant:

   ```rust
   A = 0b0001
   ```

 - Use `#[compound(...)]` to combine previously defined variants:

   ```rust
   #[compound(A | B)]
   C
   ```

 - Be automatically assigned a single-bit value **only if**
   `enable_auto_assign` is enabled

 Mixing these forms incorrectly is a compile-time error.


 ### `#[compound(...)]`

 The `#[compound]` attribute allows defining a variant in terms of other
 variants using constant expressions.

 Supported expression forms:

 - Bitwise OR (`|`)
 - Parentheses
 - Unary operators (e.g. `!`)
 - Integer literals

 Example:

 ```rust
 #[compound(A | (B | C))]
 D
 ```

 Compound expressions are:

 - Fully resolved at compile time
 - Checked for infinite recursion
 - Expanded into concrete discriminant values

 Cyclic definitions are rejected with a compile-time error.


 ### `enable_auto_assign`

 When enabled, variants without explicit values or `#[compound]` are assigned
 sequential single-bit values:

 ```rust
 A = 1 << 0
 B = 1 << 1
 C = 1 << 2
 ```

 Notes:

 - Ordering matters
 - Auto-assignment cannot be mixed with explicit discriminants

 ### Generated Types

 For an enum named `Permissions`, this macro generates:

 ```rust
 pub struct PermissionsBits(repr_type);
 ```

 Properties:

 - `#[repr(transparent)]`
 - Copyable
 - Comparable
 - Hashable

 The bits type is a **thin wrapper** around the raw integer.

 ### Operators

 The following operators are implemented:

 - Between enum variants → `PermissionsBits`
 - Between `PermissionsBits` values
 - Between enum variants and `PermissionsBits`

 Supported operators:

 - `|`, `|=`
 - `&`, `&=`
 - `^`, `^=`
 - `!`
 - `-=` (bit subtraction: `a &= !b`)


 ### Conversions

 The following conversions are provided:

 - `Permissions → PermissionsBits`
 - `PermissionsBits → repr_type`
 - `repr_type → PermissionsBits`
 - `Permissions → repr_type`

 ### Debug Formatting

 `PermissionsBits` implements `Debug` by attempting to decompose the stored
 bits into known enum variants.

 Example output:

 ```text
 PermissionsBits(Read | Write)
 ```

 If no known variants match:

 - `0` is printed as `0x0`
 - Unknown bits are printed in hexadecimal
