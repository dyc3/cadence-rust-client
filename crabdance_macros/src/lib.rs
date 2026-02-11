use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::parse::{Parse, ParseStream, Parser};
use syn::punctuated::Punctuated;
use syn::{parse_macro_input, Expr, FnArg, Ident, ItemFn, Lit, Meta, Pat, PatType, Token, Type};

fn parse_name_argument(args: TokenStream) -> syn::Result<Option<String>> {
    if args.is_empty() {
        return Ok(None);
    }

    let parser = Punctuated::<Meta, Token![,]>::parse_terminated;
    let args = parser.parse(args)?;
    for arg in args {
        if let Meta::NameValue(meta) = arg {
            if meta.path.is_ident("name") {
                if let Expr::Lit(expr_lit) = meta.value {
                    if let Lit::Str(value) = expr_lit.lit {
                        return Ok(Some(value.value()));
                    }
                }
            }
        }
    }
    Ok(None)
}

/// Mark a workflow function and generate a registry helper.
///
/// # Example
///
/// ```ignore
/// use crabdance_workflow::{workflow, WorkflowContext};
///
/// #[workflow(name = "welcome_flow")]
/// async fn welcome_flow(ctx: WorkflowContext, input: String) -> Result<(), crabdance_worker::WorkflowError> {
///     println!("Hello {}", input);
///     Ok(())
/// }
///
/// fn register_all(registry: &dyn crabdance_worker::Registry) {
///     welcome_flow_cadence::register(registry);
/// }
/// ```
#[proc_macro_attribute]
pub fn workflow(args: TokenStream, input: TokenStream) -> TokenStream {
    let name_value = match parse_name_argument(args) {
        Ok(value) => value,
        Err(err) => return err.to_compile_error().into(),
    };

    let item = parse_macro_input!(input as ItemFn);
    let vis = &item.vis;
    let sig = &item.sig;
    let block = &item.block;
    let ident = &sig.ident;
    let wrapper_ident = format_ident!("__CadenceWorkflow_{}", ident);
    let module_ident = format_ident!("{}_cadence", ident);
    let name_literal = name_value.unwrap_or_else(|| ident.to_string());

    let args = match extract_fn_args(sig) {
        Ok(args) => args,
        Err(err) => return err.to_compile_error().into(),
    };

    let ctx_ident = match args.ctx_ident {
        Some(id) => id,
        None => {
            return syn::Error::new_spanned(
                sig,
                "workflow function must accept a context argument",
            )
            .to_compile_error()
            .into();
        }
    };

    let call_args = args.call_args.clone();
    let ctx_binding = if args.ctx_by_ref {
        quote! { &ctx }
    } else {
        quote! { ctx }
    };

    let guard_bindings = args.guard_args.iter().map(|arg| {
        let name = &arg.ident;
        let ty = &arg.ty;
        quote! {
            let #name = <#ty as crabdance_core::FromResources>::get(
                #ctx_ident
                    .resource_context()
                    .ok_or_else(|| crabdance_worker::registry::WorkflowError::ExecutionFailed("resources not configured".to_string()))?
            )
            .await
            .map_err(|e| crabdance_worker::registry::WorkflowError::ExecutionFailed(e.to_string()))?;
        }
    });

    let input_decode = args.input_arg.as_ref().map(|_| {
        quote! {
            let input_data = input.ok_or_else(|| crabdance_worker::registry::WorkflowError::ExecutionFailed("Missing input".to_string()))?;
            let decoded = serde_json::from_slice(&input_data)
                .map_err(|e| crabdance_worker::registry::WorkflowError::ExecutionFailed(e.to_string()))?;
        }
    });

    let input_binding = args.input_arg.as_ref().map(|input| {
        let name = &input.ident;
        quote! {
            let #name = decoded;
        }
    });

    let expanded = quote! {
        #vis #sig #block

        #[derive(Clone)]
        #[allow(non_camel_case_types)]
        #vis struct #wrapper_ident;

        impl crabdance_worker::registry::Workflow for #wrapper_ident {
            fn execute(
                &self,
                ctx: crabdance_workflow::WorkflowContext,
                input: Option<Vec<u8>>,
            ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Vec<u8>, crabdance_worker::registry::WorkflowError>> + Send>> {
                Box::pin(async move {
                    let #ctx_ident = #ctx_binding;
                    #input_decode
                    #(#guard_bindings)*
                    #input_binding
                    let output = #ident(#(#call_args),*).await?;
                    serde_json::to_vec(&output)
                        .map_err(|e| crabdance_worker::registry::WorkflowError::ExecutionFailed(e.to_string()))
                })
            }
        }

        #vis mod #module_ident {
            pub const NAME: &str = #name_literal;

            pub fn register(registry: &dyn crabdance_worker::registry::Registry) {
                registry.register_workflow(NAME, Box::new(super::#wrapper_ident));
            }
        }
    };

    expanded.into()
}

/// Mark an activity function and generate a registry helper.
///
/// # Example
///
/// ```ignore
/// use crabdance_activity::{activity, ActivityContext};
///
/// #[activity(name = "send_email")]
/// async fn send_email(_ctx: &ActivityContext, input: String) -> Result<(), crabdance_worker::ActivityError> {
///     println!("Send {}", input);
///     Ok(())
/// }
///
/// fn register_all(registry: &dyn crabdance_worker::Registry) {
///     send_email_cadence::register(registry);
/// }
/// ```
#[proc_macro_attribute]
pub fn activity(args: TokenStream, input: TokenStream) -> TokenStream {
    let name_value = match parse_name_argument(args) {
        Ok(value) => value,
        Err(err) => return err.to_compile_error().into(),
    };

    let item = parse_macro_input!(input as ItemFn);
    let vis = &item.vis;
    let sig = &item.sig;
    let block = &item.block;
    let ident = &sig.ident;
    let wrapper_ident = format_ident!("__CadenceActivity_{}", ident);
    let module_ident = format_ident!("{}_cadence", ident);
    let name_literal = name_value.unwrap_or_else(|| ident.to_string());

    let args = match extract_fn_args(sig) {
        Ok(args) => args,
        Err(err) => return err.to_compile_error().into(),
    };

    let ctx_ident = match args.ctx_ident {
        Some(id) => id,
        None => {
            return syn::Error::new_spanned(
                sig,
                "activity function must accept a context argument",
            )
            .to_compile_error()
            .into();
        }
    };

    let call_args = args.call_args.clone();
    let ctx_binding = if args.ctx_by_ref {
        quote! { &ctx }
    } else {
        quote! { ctx }
    };

    let guard_bindings = args.guard_args.iter().map(|arg| {
        let name = &arg.ident;
        let ty = &arg.ty;
        quote! {
            let #name = <#ty as crabdance_core::FromResources>::get(
                #ctx_ident
                    .resource_context()
                    .ok_or_else(|| crabdance_worker::registry::ActivityError::Retryable("resources not configured".to_string()))?
            )
            .await
            .map_err(|e| crabdance_worker::registry::ActivityError::Retryable(e.to_string()))?;
        }
    });

    let input_decode = args.input_arg.as_ref().map(|_| {
        quote! {
            let input_data = input.ok_or_else(|| crabdance_worker::registry::ActivityError::ExecutionFailed("Missing input".to_string()))?;
            let decoded = serde_json::from_slice(&input_data)
                .map_err(|e| crabdance_worker::registry::ActivityError::ExecutionFailed(e.to_string()))?;
        }
    });

    let input_binding = args.input_arg.as_ref().map(|input| {
        let name = &input.ident;
        quote! {
            let #name = decoded;
        }
    });

    let expanded = quote! {
        #vis #sig #block

        #[derive(Clone)]
        #[allow(non_camel_case_types)]
        #vis struct #wrapper_ident;

        impl crabdance_worker::registry::Activity for #wrapper_ident {
            fn execute(
                &self,
                ctx: &crabdance_activity::ActivityContext,
                input: Option<Vec<u8>>,
            ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Vec<u8>, crabdance_worker::registry::ActivityError>> + Send>> {
                let ctx = ctx.clone();
                Box::pin(async move {
                    let #ctx_ident = #ctx_binding;
                    #input_decode
                    #(#guard_bindings)*
                    #input_binding
                    let output = #ident(#(#call_args),*).await?;
                    serde_json::to_vec(&output)
                        .map_err(|e| crabdance_worker::registry::ActivityError::ExecutionFailed(e.to_string()))
                })
            }
        }

        #vis mod #module_ident {
            pub const NAME: &str = #name_literal;

            pub fn register(registry: &dyn crabdance_worker::registry::Registry) {
                registry.register_activity(NAME, Box::new(super::#wrapper_ident));
            }
        }
    };

    expanded.into()
}

/// Call a typed activity using the name from `#[activity]`.
///
/// # Example
///
/// ```ignore
/// use crabdance_workflow::{call_activity, WorkflowContext};
///
/// async fn run(ctx: WorkflowContext) -> Result<(), crabdance_worker::WorkflowError> {
///     let options = crabdance_core::ActivityOptions::default();
///     let _: () = call_activity!(ctx, send_email, "hello".to_string(), options).await?;
///     Ok(())
/// }
/// ```
#[proc_macro]
pub fn call_activity(input: TokenStream) -> TokenStream {
    let args = parse_macro_input!(input as CallActivityArgs);
    let ctx = args.ctx;
    let payload = args.payload;
    let options = args.options;
    let mut name_path = args.activity.clone();
    if let Some(segment) = name_path.segments.last_mut() {
        segment.ident = format_ident!("{}_cadence", segment.ident);
    }
    name_path
        .segments
        .push(syn::PathSegment::from(format_ident!("NAME")));

    let expanded = quote! {
        async {
            let bytes = serde_json::to_vec(&#payload)
                .map_err(|e| crabdance_workflow::WorkflowError::ExecutionFailed(e.to_string()))?;
            let result_bytes = #ctx
                .execute_activity(
                    #name_path,
                    Some(bytes),
                    #options,
                )
                .await?;
            serde_json::from_slice(&result_bytes)
                .map_err(|e| crabdance_workflow::WorkflowError::ExecutionFailed(e.to_string()))
        }
    };

    expanded.into()
}

struct CallActivityArgs {
    ctx: Expr,
    activity: syn::Path,
    payload: Expr,
    options: Expr,
}

struct FunctionArgs {
    ctx_ident: Option<Ident>,
    ctx_by_ref: bool,
    guard_args: Vec<ArgBinding>,
    input_arg: Option<ArgBinding>,
    call_args: Vec<Ident>,
}

#[derive(Clone)]
struct ArgBinding {
    ident: Ident,
    ty: Type,
}

fn extract_fn_args(sig: &syn::Signature) -> syn::Result<FunctionArgs> {
    let mut ctx_ident = None;
    let mut ctx_by_ref = false;
    let mut guard_args = Vec::new();
    let mut input_arg = None;
    let mut call_args = Vec::new();

    for (index, arg) in sig.inputs.iter().enumerate() {
        let FnArg::Typed(PatType { pat, ty, .. }) = arg else {
            return Err(syn::Error::new_spanned(arg, "expected a typed argument"));
        };

        let Pat::Ident(pat_ident) = pat.as_ref() else {
            return Err(syn::Error::new_spanned(
                pat,
                "expected an identifier argument",
            ));
        };

        let ident = pat_ident.ident.clone();
        if index == 0 {
            if let Type::Reference(_) = &**ty {
                ctx_by_ref = true;
            }
            ctx_ident = Some(ident.clone());
            call_args.push(ident);
            continue;
        }

        guard_args.push(ArgBinding {
            ident: ident.clone(),
            ty: *ty.clone(),
        });
        call_args.push(ident);
    }

    if let Some(last_guard) = guard_args.pop() {
        input_arg = Some(last_guard);
    }

    Ok(FunctionArgs {
        ctx_ident,
        ctx_by_ref,
        guard_args,
        input_arg,
        call_args,
    })
}

impl Parse for CallActivityArgs {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let ctx: Expr = input.parse()?;
        input.parse::<Token![,]>()?;
        let activity: syn::Path = input.parse()?;
        input.parse::<Token![,]>()?;
        let payload: Expr = input.parse()?;
        input.parse::<Token![,]>()?;
        let options: Expr = input.parse()?;

        Ok(Self {
            ctx,
            activity,
            payload,
            options,
        })
    }
}
