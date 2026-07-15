use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::parse::{Parse, ParseStream};
use syn::{Expr, ExprBlock, Ident, Token};

struct NodeDecl {
    name: Ident,
    components: Vec<ComponentField>,
}

struct ComponentField {
    name: Ident,
    ty: Ident,
    default: Expr,
}

impl Parse for NodeDecl {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let name: Ident = input.parse()?;
        let _comma: Token![,] = input.parse()?;
        let content;
        let _braces = syn::braced!(content in input);

        let mut components = Vec::new();
        while !content.is_empty() {
            let key: Ident = content.parse()?;
            if key != "components" {
                return Err(syn::Error::new(key.span(), "expected `components`"));
            }
            let _colon: Token![:] = content.parse()?;
            let comp_content;
            let _inner_braces = syn::braced!(comp_content in content);

            while !comp_content.is_empty() {
                let field_name: Ident = comp_content.parse()?;
                let _colon: Token![:] = comp_content.parse()?;
                let field_ty: Ident = comp_content.parse()?;
                let _eq: Token![=] = comp_content.parse()?;
                let default: Expr = comp_content.parse()?;
                let _trailing: Option<Token![,]> = comp_content.parse().ok();
                components.push(ComponentField {
                    name: field_name,
                    ty: field_ty,
                    default,
                });
            }

            let _trailing: Option<Token![,]> = content.parse().ok();
        }

        Ok(NodeDecl { name, components })
    }
}

#[proc_macro]
pub fn craft_node(input: TokenStream) -> TokenStream {
    let decl = match syn::parse2::<NodeDecl>(input.into()) {
        Ok(d) => d,
        Err(e) => return e.to_compile_error().into(),
    };

    let type_name = decl.name.to_string();
    let specs = decl.components.iter().map(|c| {
        let field_name = c.name.to_string();
        let ty_ident = format_ident!("{}", c.ty);
        let default_expr = &c.default;
        quote! {
            ::craft_kernel::scene::ComponentSpec::new(
                #field_name,
                ::craft_kernel::scene::ComponentType::#ty_ident,
                serde_json::json!(#default_expr),
            )
        }
    });

    let type_name_lit = type_name.as_str();

    let ident = decl.name;
    quote! {
        #[allow(non_camel_case_types)]
        #[derive(Debug, Default, Clone, Copy)]
        pub struct #ident;

        impl ::craft_kernel::scene::NodeDef for #ident {
            fn type_name(&self) -> &'static str {
                #type_name_lit
            }

            fn component_specs(&self) -> Vec<::craft_kernel::scene::ComponentSpec> {
                vec![ #( #specs ),* ]
            }
        }

        ::craft_kernel::inventory::submit!(::craft_kernel::scene::NodeRegistration {
            name: #type_name_lit,
            instantiate: || -> Box<dyn ::craft_kernel::scene::NodeDef> {
                Box::new(#ident)
            },
        });
    }
    .into()
}

fn _unused() {}

struct SystemDecl {
    name: Ident,
    phase: Ident,
    body: ExprBlock,
}

impl Parse for SystemDecl {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let name: Ident = input.parse()?;
        let _comma: Token![,] = input.parse()?;
        let phase_key: Ident = input.parse()?;
        if phase_key != "phase" {
            return Err(syn::Error::new(
                phase_key.span(),
                "expected `phase:` in craft_system!",
            ));
        }
        let _colon: Token![:] = input.parse()?;
        let phase: Ident = input.parse()?;
        let _comma: Token![,] = input.parse()?;
        let body: ExprBlock = input.parse()?;
        Ok(SystemDecl { name, phase, body })
    }
}

#[proc_macro]
pub fn craft_system(input: TokenStream) -> TokenStream {
    let decl = match syn::parse2::<SystemDecl>(input.into()) {
        Ok(d) => d,
        Err(e) => return e.to_compile_error().into(),
    };

    let name = decl.name;
    let name_str = name.to_string();
    let phase = decl.phase;
    let body = &decl.body;

    quote! {
        #[allow(non_camel_case_types)]
        #[derive(Debug, Clone, Copy)]
        pub struct #name;

        impl ::craft_kernel::system::System for #name {
            fn info(&self) -> ::craft_kernel::system::SystemInfo {
                ::craft_kernel::system::SystemInfo {
                    name: #name_str,
                    phase: ::craft_kernel::system::SystemPhase::#phase,
                }
            }

            fn run(&mut self, ctx: &mut ::craft_kernel::system::SystemContext<'_>) {
                let _ = &mut *ctx;
                #body
            }
        }

        ::craft_kernel::inventory::submit!(::craft_kernel::system::SystemRegistration {
            info: ::craft_kernel::system::SystemInfo {
                name: #name_str,
                phase: ::craft_kernel::system::SystemPhase::#phase,
            },
            instantiate: || -> Box<dyn ::craft_kernel::system::System> {
                Box::new(#name)
            },
        });
    }
    .into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_simple_ident() {
        let src = quote::quote! { Foo };
        let name: Ident = syn::parse2(src).unwrap();
        assert_eq!(name.to_string(), "Foo");
    }

    #[test]
    fn parses_minimal_node_decl() {
        let src = quote::quote! {
            Player, {
                components: {
                    health: Int = 100,
                },
            }
        };
        let decl: NodeDecl = syn::parse2(src).expect("parse NodeDecl");
        assert_eq!(decl.name.to_string(), "Player");
        assert_eq!(decl.components.len(), 1);
        assert_eq!(decl.components[0].name.to_string(), "health");
        assert_eq!(decl.components[0].ty.to_string(), "Int");
    }

    #[test]
    fn parses_node_decl_with_array_default() {
        let src = quote::quote! {
            Player, {
                components: {
                    position: Vec2 = [0.0, 0.0],
                },
            }
        };
        let decl: NodeDecl = syn::parse2(src).expect("parse NodeDecl");
        assert_eq!(decl.components.len(), 1);
        assert_eq!(decl.components[0].name.to_string(), "position");
        assert_eq!(decl.components[0].ty.to_string(), "Vec2");
    }
}
