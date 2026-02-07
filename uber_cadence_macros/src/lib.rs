use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::parse::{Parse, ParseStream, Parser};
use syn::punctuated::Punctuated;
use syn::{parse_macro_input, Expr, ItemFn, Lit, Meta, Token};

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
/// ```rust
/// use uber_cadence_workflow::{workflow, WorkflowContext};
///
/// #[workflow(name = "welcome_flow")]
/// async fn welcome_flow(ctx: WorkflowContext, input: String) -> Result<(), uber_cadence_worker::WorkflowError> {
///     println!("Hello {}", input);
///     Ok(())
/// }
///
/// fn register_all(registry: &dyn uber_cadence_worker::Registry) {
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

    let expanded = quote! {
        #vis #sig #block

        #[derive(Clone)]
        #[allow(non_camel_case_types)]
        #vis struct #wrapper_ident;

        impl uber_cadence_worker::registry::Workflow for #wrapper_ident {
            fn execute(
                &self,
                ctx: uber_cadence_workflow::WorkflowContext,
                input: Option<Vec<u8>>,
            ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Vec<u8>, uber_cadence_worker::registry::WorkflowError>> + Send>> {
                Box::pin(async move {
                    let input_data = input.ok_or_else(|| uber_cadence_worker::registry::WorkflowError::ExecutionFailed("Missing input".to_string()))?;
                    let decoded = serde_json::from_slice(&input_data)
                        .map_err(|e| uber_cadence_worker::registry::WorkflowError::ExecutionFailed(e.to_string()))?;
                    let output = #ident(ctx, decoded).await?;
                    serde_json::to_vec(&output)
                        .map_err(|e| uber_cadence_worker::registry::WorkflowError::ExecutionFailed(e.to_string()))
                })
            }
        }

        #vis mod #module_ident {
            pub const NAME: &str = #name_literal;

            pub fn register(registry: &dyn uber_cadence_worker::registry::Registry) {
                registry.register_workflow(NAME, Box::new(super::#wrapper_ident));
            }
        }
    };

    expanded.into()
}

/// Mark an activity function and generate a registry helper.
///
/// ```rust
/// use uber_cadence_activity::{activity, ActivityContext};
///
/// #[activity(name = "send_email")]
/// async fn send_email(_ctx: &ActivityContext, input: String) -> Result<(), uber_cadence_worker::ActivityError> {
///     println!("Send {}", input);
///     Ok(())
/// }
///
/// fn register_all(registry: &dyn uber_cadence_worker::Registry) {
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

    let expanded = quote! {
        #vis #sig #block

        #[derive(Clone)]
        #[allow(non_camel_case_types)]
        #vis struct #wrapper_ident;

        impl uber_cadence_worker::registry::Activity for #wrapper_ident {
            fn execute(
                &self,
                ctx: &uber_cadence_activity::ActivityContext,
                input: Option<Vec<u8>>,
            ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Vec<u8>, uber_cadence_worker::registry::ActivityError>> + Send>> {
                let ctx = ctx.clone();
                Box::pin(async move {
                    let input_data = input.ok_or_else(|| uber_cadence_worker::registry::ActivityError::ExecutionFailed("Missing input".to_string()))?;
                    let decoded = serde_json::from_slice(&input_data)
                        .map_err(|e| uber_cadence_worker::registry::ActivityError::ExecutionFailed(e.to_string()))?;
                    let output = #ident(&ctx, decoded).await?;
                    serde_json::to_vec(&output)
                        .map_err(|e| uber_cadence_worker::registry::ActivityError::ExecutionFailed(e.to_string()))
                })
            }
        }

        #vis mod #module_ident {
            pub const NAME: &str = #name_literal;

            pub fn register(registry: &dyn uber_cadence_worker::registry::Registry) {
                registry.register_activity(NAME, Box::new(super::#wrapper_ident));
            }
        }
    };

    expanded.into()
}

/// Call a typed activity using the name from `#[activity]`.
///
/// ```rust
/// use uber_cadence_workflow::{call_activity, WorkflowContext};
///
/// async fn run(ctx: WorkflowContext) -> Result<(), uber_cadence_worker::WorkflowError> {
///     let options = uber_cadence_core::ActivityOptions::default();
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
                .map_err(|e| uber_cadence_workflow::WorkflowError::ExecutionFailed(e.to_string()))?;
            let result_bytes = #ctx
                .execute_activity(
                    #name_path,
                    Some(bytes),
                    #options,
                )
                .await?;
            serde_json::from_slice(&result_bytes)
                .map_err(|e| uber_cadence_workflow::WorkflowError::ExecutionFailed(e.to_string()))
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
