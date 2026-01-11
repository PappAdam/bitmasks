use std::collections::HashMap;

use proc_macro::TokenStream;
use proc_macro2::Span;
use syn::{Attribute, Error, Expr, Ident, ItemEnum, Variant, parse_macro_input};

use crate::{derive::derive, resolve_masks::resolve_variant};

mod derive;
mod resolve_masks;

/// ## `#[bitmask]`
///
/// Declares a *bitmask definition* using an `enum` as a **compile-time DSL** and
/// generates a corresponding transparent *bits type* for runtime use.
///
/// This macro is intentionally **low-level**. It does **not** attempt to model
/// permissions, states, or invariants. It only provides:
///
/// - Explicit bit values
/// - Compile-time composition of bits
/// - A thin, transparent wrapper around the raw integer
///
/// ### Overview
///
/// Given an enum definition:
///
/// ```rust
/// #[bitmask(enable_auto_assign)]
/// #[repr(u8)]
/// pub enum Permissions {
///     Read,
///     Write,
///     #[compound(Read | Write)]
///     ReadWrite,
/// }
/// ```
///
/// This macro:
///
/// 1. Resolves all enum discriminants into concrete bit expressions
/// 2. Expands `#[compound(...)]` expressions at compile time
/// 3. Generates a transparent `PermissionsBits` wrapper around the underlying
///    integer representation
/// 4. Implements standard bitwise operators between:
///    - enum variants
///    - the generated bits type
///    - the raw integer representation
///
/// The original enum is preserved and usable, but it should be understood as
/// **a named source of bit patterns**, not a closed set of states.
///
///
/// ### Representation
///
/// A concrete integer representation is **required**:
///
/// ```rust
/// #[repr(u8 | u16 | u32 | u64 | u128 | usize)]
/// ```
///
/// Signed integer representations are **not supported**.
/// Bitmasks are defined in terms of unsigned bitwise operations only.
///
///
/// ### Variant Assignment Rules
///
/// Each enum variant must satisfy **exactly one** of the following:
///
/// - Have an explicit discriminant:
///
///   ```rust
///   A = 0b0001
///   ```
///
/// - Use `#[compound(...)]` to combine previously defined variants:
///
///   ```rust
///   #[compound(A | B)]
///   C
///   ```
///
/// - Be automatically assigned a single-bit value **only if**
///   `enable_auto_assign` is enabled
///
/// Mixing these forms incorrectly is a compile-time error.
///
///
/// ### `#[compound(...)]`
///
/// The `#[compound]` attribute allows defining a variant in terms of other
/// variants using constant expressions.
///
/// Supported expression forms:
///
/// - Bitwise OR (`|`)
/// - Parentheses
/// - Unary operators (e.g. `!`)
/// - Integer literals
///
/// Example:
///
/// ```rust
/// #[compound(A | (B | C))]
/// D
/// ```
///
/// Compound expressions are:
///
/// - Fully resolved at compile time
/// - Checked for infinite recursion
/// - Expanded into concrete discriminant values
///
/// Cyclic definitions are rejected with a compile-time error.
///
///
/// ### `enable_auto_assign`
///
/// When enabled, variants without explicit values or `#[compound]` are assigned
/// sequential single-bit values:
///
/// ```rust
/// A = 1 << 0
/// B = 1 << 1
/// C = 1 << 2
/// ```
///
/// Notes:
///
/// - Ordering matters
/// - Auto-assignment cannot be mixed with explicit discriminants
///
/// ### Generated Types
///
/// For an enum named `Permissions`, this macro generates:
///
/// ```rust
/// pub struct PermissionsBits(repr_type);
/// ```
///
/// Properties:
///
/// - `#[repr(transparent)]`
/// - Copyable
/// - Comparable
/// - Hashable
///
/// The bits type is a **thin wrapper** around the raw integer.
///
/// ### Operators
///
/// The following operators are implemented:
///
/// - Between enum variants → `PermissionsBits`
/// - Between `PermissionsBits` values
/// - Between enum variants and `PermissionsBits`
///
/// Supported operators:
///
/// - `|`, `|=`
/// - `&`, `&=`
/// - `^`, `^=`
/// - `!`
/// - `-=` (bit subtraction: `a &= !b`)
///
///
/// ### Conversions
///
/// The following conversions are provided:
///
/// - `Permissions → PermissionsBits`
/// - `PermissionsBits → repr_type`
/// - `repr_type → PermissionsBits`
/// - `Permissions → repr_type`
///
/// All conversions are lossless and unchecked.
///
/// ### Debug Formatting
///
/// `PermissionsBits` implements `Debug` by attempting to decompose the stored
/// bits into known enum variants.
///
/// Example output:
///
/// ```text
/// PermissionsBits(Read | Write)
/// ```
///
/// If no known variants match:
///
/// - `0` is printed as `0x0`
/// - Unknown bits are printed in hexadecimal
///
/// ### Important Semantics
///
/// - This macro does **not** enforce exclusivity
/// - Multiple variants may overlap
/// - Not all bit patterns correspond to enum variants
/// - Pattern matching on the enum does **not** imply exhaustiveness
///
/// The enum is best understood as a *named bit catalog*, not a state machine.
#[proc_macro_attribute]
pub fn bitmask(attr: TokenStream, item: TokenStream) -> TokenStream {
    let mut input = parse_macro_input!(item as ItemEnum);
    let mut all_errors: Option<Error> = None;
    let name = &input.ident;
    let vis = &input.vis;

    let repr = check_repr(&input.attrs);
    if let Err(e) = repr {
        return e.into_compile_error().into();
    }
    let repr = repr.unwrap();

    let mut enable_auto = false;

    let parser = syn::meta::parser(|meta| {
        if meta.path.is_ident("enable_auto_assign") {
            enable_auto = true;
            Ok(())
        } else if meta.path.is_ident("default") {
            Ok(())
        } else {
            Err(meta.error("unsupported bitmasks property"))
        }
    });

    parse_macro_input!(attr with parser);

    let mut variants: Vec<Variant> = input.variants.into_iter().collect();
    let mut resolved_values = HashMap::<Ident, Expr>::new();

    let mut compound_idxs: Vec<(usize, Attribute)> = Vec::new();
    let mut shift = 0;
    for (i, variant) in variants.iter_mut().enumerate() {
        let comp_idx = variant
            .attrs
            .iter()
            .position(|a| a.path().is_ident("compound"));

        if let Some((_, expr)) = &variant.discriminant {
            if enable_auto {
                let e = syn::Error::new_spanned(
                    &variant.ident,
                    "Conflict: Remove enable_auto_assign to manually assign values",
                );
                return e.into_compile_error().into();
            } else if let Some(_) = comp_idx {
                let e = syn::Error::new_spanned(
                    &variant.ident,
                    "Conflict: Variant has both a explicit value and a #[compound] attribute.",
                );
                return e.into_compile_error().into();
            }
            resolved_values.insert(variant.ident.clone(), expr.clone());
        } else {
            if let None = comp_idx {
                if enable_auto {
                    let expr: Expr = syn::parse_quote!(1 << #shift);
                    shift += 1;
                    resolved_values.insert(variant.ident.clone(), expr.clone());
                    variant.discriminant = Some((Default::default(), expr.clone()));
                } else {
                    let e = syn::Error::new_spanned(
                        &variant.ident,
                        "Variant should have either an explicit value or a #[compound(...)] attribute.",
                    );
                    return e.into_compile_error().into();
                }
            }
        }

        if let Some(idx) = comp_idx {
            compound_idxs.push((i, variant.attrs.remove(idx)));
        }
    }

    for (i, attr) in compound_idxs {
        let mut computed_idents: Vec<Ident> = Vec::new();
        let resolve_variant = resolve_variant(
            i,
            attr,
            &mut variants,
            &mut resolved_values,
            &mut computed_idents,
        );
        if let Err(e) = resolve_variant {
            match &mut all_errors {
                Some(existing_error) => existing_error.combine(e),
                None => all_errors = Some(e),
            }
        }
    }

    if let Some(e) = all_errors {
        return e.to_compile_error().into();
    }

    input.variants = variants.into_iter().collect();

    TokenStream::from(derive(&input, vis, name, &repr))
}

fn check_repr(attrs: &[Attribute]) -> Result<Ident, syn::Error> {
    attrs
        .iter()
        .filter(|a| a.path().is_ident("repr"))
        .find_map(|a| {
            let mut repr_type = None;
            let res = a
                .parse_nested_meta(|meta| {
                    if meta.path.is_ident("C") {
                        return Ok(());
                    }
                    if meta.path.is_ident("u8")
                        || meta.path.is_ident("u16")
                        || meta.path.is_ident("u32")
                        || meta.path.is_ident("u64")
                        || meta.path.is_ident("u128")
                        || meta.path.is_ident("usize")
                    {
                        repr_type = Some(meta.path.get_ident().unwrap().clone())
                    }
                    Ok(())
                })
                .map_err(|_| false);

            match res {
                Ok(_) => repr_type,
                Err(_) => None,
            }
        })
        .ok_or(syn::Error::new(
            Span::call_site(),
            "Bitmasks require a explicitly defined representation. Please add #[repr(u32)], #[repr(u64)] etc. (automatic #[repr(C)] might be added in the future)",
        ))
}
