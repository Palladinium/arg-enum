extern crate proc_macro;

use proc_macro2::{Span, TokenStream as TokenStream2};
use quote::quote;
use syn::{
    parenthesized,
    parse::{Parse, ParseStream},
    parse2, parse_macro_input,
    punctuated::Punctuated,
    spanned::Spanned,
    token::Paren,
    Attribute, Data, Error, Fields, Ident, LitStr, Result, Token,
};

mod case;

mod a {
    pub const BASE: &str = "arg_enum";

    pub const RENAME: &str = "rename";
    pub const RENAME_ALL: &str = "rename_all";
    pub const DEFAULT: &str = "default";
}

#[proc_macro_derive(ArgEnum, attributes(arg_enum))]
pub fn derive_arg_enum(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as syn::DeriveInput);

    let output = arg_enum_type(input).unwrap_or_else(|e| e.to_compile_error());

    proc_macro::TokenStream::from(output)
}

fn arg_enum_type(input: syn::DeriveInput) -> Result<TokenStream2> {
    let span = input.span();

    let e = match &input.data {
        Data::Enum(e) => e,
        _ => return Err(Error::new(span, "#[derive(ArgEnum)] only works on enums")),
    };

    let ident = input.ident;
    let ident_s = LitStr::new(&ident.to_string(), Span::call_site());

    let rename_all = get_attr(&input.attrs, a::RENAME_ALL)?
        .map(|s| case::RenameRule::from_str(&s).map_err(|e| Error::new(span, e)))
        .transpose()?;

    let mut default_variant = None;

    let variants = e
        .variants
        .iter()
        .map(|v| {
            match v.fields {
                Fields::Unit => {}
                _ => {
                    return Err(Error::new(
                        v.span(),
                        "#[derive(ArgEnum)] only works on unit enum variants",
                    ))
                }
            }

            let s = get_attr(&v.attrs, a::RENAME)?
                .or_else(|| rename_all.map(|ra| ra.apply_to_variant(&v.ident.to_string())))
                .unwrap_or_else(|| v.ident.to_string());

            if has_attr(&v.attrs, a::DEFAULT)? {
                if let Some(prev_default) = default_variant.replace(&v.ident) {
                    return Err(Error::new(
                        v.ident.span(),
                        format!(
                            "Duplicate default value for {}: {} and {}",
                            ident, prev_default, &v.ident
                        ),
                    ));
                }
            }

            Ok((&v.ident, LitStr::new(&s, Span::call_site())))
        })
        .collect::<Result<Vec<_>>>()?;

    let from_str_arms = variants.iter().map(|(v_ident, v_rename)| {
        quote! {
            #v_rename => Ok(#ident::#v_ident),
        }
    });

    let to_str_arms = variants.iter().map(|(v_ident, v_rename)| {
        quote! {
            #ident::#v_ident => #v_rename,
        }
    });

    let default_impl = if let Some(default_variant) = default_variant {
        quote! {
            impl Default for #ident {
                fn default() -> Self {
                    #ident::#default_variant
                }
            }
        }
    } else {
        TokenStream2::new()
    };

    let possible_values = variants.iter().map(|(_, v_rename)| v_rename);

    Ok(quote! {
        #[automatically_derived]
        impl ::std::str::FromStr for #ident {
            type Err = String; // TODO Make this a proper error type

            fn from_str(s: &str) -> Result<Self, Self::Err> {
                match s {
                    #(#from_str_arms)*

                    _ => Err(format!("Invalid {} value: {}", #ident_s, s)),
                }
            }
        }

        #[automatically_derived]
        impl ::std::fmt::Display for #ident {
            fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
                let s = match self {
                    #(#to_str_arms)*
                };

                f.write_str(s)
            }
        }

        #default_impl

        #[automatically_derived]
        impl #ident {
            pub const fn possible_values() -> &'static[&'static str] {
                &[
                    #(#possible_values),*
                ]
            }
        }
    })
}

struct AttrsData {
    _paren: Paren,
    values: Punctuated<AttrsValue, Token![,]>,
}

impl Parse for AttrsData {
    fn parse(input: ParseStream) -> Result<Self> {
        let content;

        // Remove when fixed https://github.com/rust-lang/rust-clippy/issues/4637
        #[allow(clippy::eval_order_dependence)]
        Ok(Self {
            _paren: parenthesized!(content in input),
            values: content.parse_terminated(AttrsValue::parse)?,
        })
    }
}

struct AttrsValue {
    key: Ident,
    _eq: Option<Token![=]>,
    value: Option<LitStr>,
}

impl Parse for AttrsValue {
    fn parse(input: ParseStream) -> Result<Self> {
        let key: Ident = input.parse()?;
        let _eq: Option<Token![=]> = input.parse()?;

        let value: Option<LitStr> = if _eq.is_some() {
            Some(input.parse()?)
        } else {
            None
        };

        Ok(Self { key, _eq, value })
    }
}

fn has_attr(attributes: &[Attribute], query: &str) -> Result<bool> {
    for attr in attributes.iter() {
        if attr.path.is_ident(a::BASE) {
            {
                let data: AttrsData = parse2(attr.tokens.clone())?;

                for kv in data.values {
                    let key = kv.key.to_string();

                    if key == query {
                        if let Some(value) = &kv.value {
                            return Err(Error::new(
                                value.span(),
                                format!("Unexpected value for attribute {}", key),
                            ));
                        }

                        return Ok(true);
                    }
                }
            }
        }
    }

    Ok(false)
}

fn get_attr(attributes: &[Attribute], query: &str) -> Result<Option<String>> {
    for attr in attributes.iter() {
        if attr.path.is_ident(a::BASE) {
            {
                let data: AttrsData = parse2(attr.tokens.clone())?;

                for kv in data.values {
                    let key = kv.key.to_string();
                    let span = kv.key.span();

                    if key == query {
                        return Ok(Some(
                            kv.value
                                .ok_or_else(|| {
                                    Error::new(span, format!("Missing value for attribute {}", key))
                                })?
                                .value(),
                        ));
                    }
                }
            }
        }
    }
    Ok(None)
}
