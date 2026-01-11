use std::collections::HashMap;

use syn::{Attribute, Expr, ExprBinary, Ident, Variant};

pub fn resolve_variant(
    i: usize,
    attr: Attribute,
    variants: &mut [Variant],
    resolved_values: &mut HashMap<Ident, Expr>,
    computed_idents: &mut Vec<Ident>,
) -> Result<Expr, syn::Error> {
    let ident = &variants[i].ident.clone();
    if computed_idents.iter().find(|i| *ident == **i).is_some() {
        return Err(syn::Error::new_spanned(
            ident,
            "Infinite recursion detected",
        ));
    }

    computed_idents.push(ident.clone());

    if let Some(expr) = resolved_values.get(ident) {
        return Ok(expr.clone());
    }

    let expr = resolve_expr(
        parse_compound(&attr)?,
        ident,
        attr.clone(),
        variants,
        resolved_values,
        computed_idents,
    )?;

    resolved_values.insert(ident.clone(), expr.clone());
    variants[i].discriminant = Some((Default::default(), expr.clone()));
    Ok(expr)
}

pub fn resolve_expr(
    expr: Expr,
    target_ident: &Ident,
    attr: Attribute,
    variants: &mut [Variant],
    resolved_values: &mut HashMap<Ident, Expr>,
    computed_idents: &mut Vec<Ident>,
) -> Result<Expr, syn::Error> {
    match expr {
        Expr::Binary(ExprBinary {
            left, op, right, ..
        }) => {
            // if !matches!(op, BinOp::BitOr(_)) {
            //     panic!("only | is supported in #[compound]");
            // }

            let left = resolve_expr(
                *left,
                target_ident,
                attr.clone(),
                variants,
                resolved_values,
                computed_idents,
            )?;
            let right = resolve_expr(
                *right,
                target_ident,
                attr.clone(),
                variants,
                resolved_values,
                computed_idents,
            )?;

            let expr: Expr = syn::parse_quote!(#left #op #right);
            Ok(expr)
        }

        Expr::Path(p) => {
            let ident = p
                .path
                .get_ident()
                .ok_or(syn::Error::new_spanned(&p, "Expected expression"))?;

            let i =
                variants
                    .iter()
                    .position(|v| v.ident == *ident)
                    .ok_or(syn::Error::new_spanned(
                        &ident,
                        &format!("No field found with name: {ident}"),
                    ))?;

            resolve_variant(i, attr.clone(), variants, resolved_values, computed_idents)
        }

        Expr::Lit(_) => Ok(expr),

        Expr::Paren(paren) => {
            let inner_resolved = resolve_expr(
                *paren.expr,
                target_ident,
                attr,
                variants,
                resolved_values,
                computed_idents,
            )?;

            Ok(inner_resolved)
        }

        Expr::Unary(u) => {
            let inner = resolve_expr(
                *u.expr,
                target_ident,
                attr,
                variants,
                resolved_values,
                computed_idents,
            )?;
            let op = u.op;
            Ok(syn::parse_quote!(#op #inner))
        }

        _ => Err(syn::Error::new_spanned(&attr, "Unsupported expression")),
    }
}

fn parse_compound(attr: &Attribute) -> Result<Expr, syn::Error> {
    attr.parse_args::<Expr>().map_err(|e| {
        syn::Error::new_spanned(
            &attr,
            &format!(
                "parsing attribute ({:?}) failed with error: {:?}",
                attr.path().get_ident(),
                e
            ),
        )
    })
}
