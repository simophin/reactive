use convert_case::{Case, Casing};
use proc_macro2::{Ident, Span, TokenStream};
use proc_macro_error::abort;
use quote::quote;
use syn::{
    parse2, parse_quote, punctuated::Punctuated, FnArg, Generics, ItemFn, Lifetime, Pat, PatType,
    Path, Token, Type, TypeParamBound, TypePath,
};

pub fn component(_attr: TokenStream, mut item: TokenStream) -> TokenStream {
    let ItemFn { vis, sig, .. } =
        parse2::<ItemFn>(item.clone()).expect("Expect a function as component");

    let component_name = Ident::new(
        &sig.ident
            .to_string()
            .from_case(Case::Snake)
            .to_case(Case::Pascal),
        sig.ident.span(),
    );

    if sig.asyncness.is_some() {
        abort!(sig.asyncness, "Component function cannot be async");
    }

    if sig.variadic.is_some() {
        abort!(sig.variadic, "Component function cannot be variadic");
    }

    let mut inputs = sig.inputs.iter();

    // First argument must be a reference to SetupContext
    match inputs.next() {
        Some(FnArg::Typed(p)) if matches!(p.ty.as_ref(), &Type::Reference(_)) => {
            let Type::Reference(r) = p.ty.as_ref() else {
                panic!("Expected reference type to SetupContext");
            };

            if r.lifetime.is_some() {
                abort!(r.lifetime, "Lifetime is not supported on SetupContext");
            }

            let Type::Path(v) = r.elem.as_ref() else {
                abort!(r.elem, "Expected type SetupContext");
            };

            match v.path.segments.last() {
                Some(s) if s.ident.to_string() == "SetupContext" && s.arguments.is_empty() => {}
                p => abort!(p, "Expected type SetupContext"),
            }
        }
        Some(v) => abort!(v, "First argument must be a reference to SetupContext"),
        None => abort!(sig, "First argument must be a reference to SetupContext"),
    }

    // let mut properties = Vec::new();
    // The rest of the inputs will be treated as properties
    // for input in inputs {
    //     let PatType { pat, ty, .. } = match input {
    //         FnArg::Typed(p) => p,
    //         v => abort!(v, "Expected a typed argument"),
    //     };

    //     let name = match pat.as_ref() {
    //         Pat::Ident(i) => i.ident.clone(),
    //         v => abort!(v, "Expected an identifier"),
    //     };

    //     properties.push((name, ty.clone()));
    // }

    let mut generics = sig.generics;
    let mut fields = vec![];
    let mut call_params = vec![];

    for a in inputs {
        let FnArg::Typed(arg) = a else {
            abort!(a, "Expected a typed argument");
        };

        let Pat::Ident(ident) = arg.pat.as_ref() else {
            abort!(arg.pat, "Expected a name");
        };

        let ty = match arg.ty.as_ref() {
            Type::Path(_) | Type::ImplTrait(_) | Type::Reference(_) => {
                fn_arg_type_to_field(arg.ty.as_ref(), &mut generics)
            }
            v => abort!(v, "Unsupported argument type in component function. Only references, paths and impl traits are supported"),
        };

        fields.push(PatType {
            attrs: Vec::new(),
            pat: arg.pat.clone(),
            colon_token: Default::default(),
            ty,
        });

        call_params.push(quote! { self.#ident });
    }

    let where_clauses = &generics.where_clause;
    let mut generics_without_bounds = generics.clone();
    generics_without_bounds
        .params
        .iter_mut()
        .for_each(|p| match p {
            syn::GenericParam::Type(t) => t.bounds = Punctuated::new(),
            _ => {}
        });

    let fun_name = &sig.ident;

    let component_tokens = quote! {
        #[derive(derive_builder::Builder)]
        #[builder(pattern = "owned")]
        #vis struct #component_name #generics #where_clauses {
            #( #fields, )*
        }

        impl #generics reactive_core::Component for #component_name #generics_without_bounds #where_clauses {
            fn setup(self: Box<Self>, ctx: &mut reactive_core::SetupContext) {
                #fun_name(ctx, #(#call_params,)*);
            }
        }
    };

    item.extend(component_tokens);

    item
}

fn fn_arg_type_to_field(pat_type: &Type, g: &mut Generics) -> Box<Type> {
    match pat_type {
        Type::Reference(r) => {
            let mut r = r.clone();
            r.lifetime
                .get_or_insert_with(|| g.get_or_create_default_lifetime());
            r.elem = fn_arg_type_to_field(&r.elem, g);
            Box::new(Type::Reference(r))
        }

        Type::Path(p) => Box::new(Type::Path(p.clone())),
        Type::ImplTrait(t) => Box::new(Type::Path(TypePath {
            qself: None,
            path: g.add_new_generic_params(&t.bounds),
        })),

        Type::TraitObject(_) => Box::new(pat_type.clone()),
        _ => abort!(pat_type, "Unsupported type in component function"),
    }
}

trait GenericsExt {
    fn get_or_create_default_lifetime(&mut self) -> Lifetime;
    fn add_new_generic_params(&mut self, bounds: &Punctuated<TypeParamBound, Token![+]>) -> Path;
    fn find_usable_generic_path(&self) -> Path;
}

impl GenericsExt for Generics {
    fn get_or_create_default_lifetime(&mut self) -> Lifetime {
        self.lifetimes()
            .next()
            .map(|s| s.lifetime.clone())
            .filter(|lifetime| lifetime.ident != "'__builder_default")
            .unwrap_or_else(|| {
                let lifetime = Lifetime::new("'__builder_default", Span::call_site());
                self.params.insert(0, parse_quote! { #lifetime });
                lifetime
            })
    }

    fn find_usable_generic_path(&self) -> Path {
        let mut curr = vec![b'A'];

        while self
            .params
            .iter()
            .any(|p| matches!(p, syn::GenericParam::Type(t) if t.ident == std::str::from_utf8(&curr).unwrap()))
        {
            match curr.last_mut().unwrap() {
                b'Z' => curr.push(b'A'),
                c => *c = c.wrapping_add(1),
            }
        }

        Path::from(Ident::new(
            std::str::from_utf8(&curr).unwrap(),
            Span::call_site(),
        ))
    }

    fn add_new_generic_params(&mut self, bounds: &Punctuated<TypeParamBound, Token![+]>) -> Path {
        let path = self.find_usable_generic_path();
        self.params.push(parse_quote! { #path: #bounds });
        path
    }
}

#[cfg(test)]
mod tests {

    use syn::parse_file;

    use super::*;

    #[test]
    fn parsing_works() {
        let input = quote! {
            pub fn content(ctx: &mut SetupContext, body: impl Signal<Value = String>) {
                let ResourceResult {
                    mut trigger,
                    state,
                    update,
                } = ctx.create_resource((), |_| async move {
                    sleep(Duration::from_secs(10)).await;
                    "Future result"
                });

                let theme = ctx.require_context(&THEME);

                ctx.create_effect_simple(move || {
                    println!("Future load result: {:?}", state.get());

                    if state.with(|v| v.state) == LoadState::Loaded {
                        println!("Reload result");
                        trigger();
                    }
                });

                ctx.create_effect_simple(move || {
                    theme.with(|v| println!("Theme = {v}"));
                });

                ctx.on_clean_up(|| {
                    println!("content clean up");
                });
            }
        };

        let output = component(Default::default(), input);

        let output = prettyplease::unparse(&parse_file(&output.to_string()).unwrap());
        println!("{output}");
    }
}
