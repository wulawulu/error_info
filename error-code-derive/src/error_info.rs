use darling::{
    ast::{Data, Fields, Style},
    util, FromDeriveInput, FromVariant,
};
use proc_macro2::TokenStream;
use quote::quote;
use syn::DeriveInput;

#[derive(Debug, FromDeriveInput)]
#[darling(attributes(error_info))]
struct ErrorData {
    ident: syn::Ident,
    generics: syn::Generics,
    data: Data<ErrorVariant, ()>,
    app_type: syn::Type,
    #[darling(default)]
    prefix: String,
}

#[derive(Debug, FromVariant)]
#[darling(attributes(error_info))]
struct ErrorVariant {
    ident: syn::Ident,
    fields: Fields<util::Ignored>,
    #[darling(default)]
    code: String,
    #[darling(default)]
    app_code: String,
    #[darling(default)]
    client_msg: String,
}

pub(crate) fn process_error_info(input: DeriveInput) -> TokenStream {
    let ErrorData {
        ident: name,
        generics,
        data: Data::Enum(data),
        app_type,
        prefix,
    } = ErrorData::from_derive_input(&input).expect("Can not parse input")
    else {
        panic!("ErrorInfo only supports enum");
    };

    let code = data
        .iter()
        .map(|v| {
            let ErrorVariant {
                ident,
                fields,
                code,
                app_code,
                client_msg,
            } = v;
            let variant_code = match fields.style {
                Style::Tuple => quote! { #name::#ident(_) },
                Style::Struct => quote! { #name::#ident{..} },
                Style::Unit => quote! { #name::#ident },
            };
            let code = format!("{}{}", prefix, code);
            quote! {
                #variant_code => {
                    ErrorInfo::new(
                    #app_code,
                    #code,
                    #client_msg,
                    self,
                    )
                }
            }
        })
        .collect::<Vec<_>>();

    quote! {
        use error_code::{ErrorInfo, ToErrorInfo as _};
        impl #generics ToErrorInfo for #name #generics {
            type T = #app_type;

            fn to_error_info(&self) -> ErrorInfo<Self::T> {
                match self {
                    #(#code),*
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_data_struct() {
        let input = r#"
        #[derive(thiserror::Error, ToErrorInfo)]
        #[error_info(app_type="StatusCode", prefix="01")]
        pub enum MyError {
        #[error("Invalid command: {0}")]
        #[error_info(code="IC", app_code="400")]
        InvalidCommand(String),
        #[error("Invalid argument: {0}")]
        #[error_info(code="IA", app_code="400", client_msg="friendly msg")]
        InvalidArgument(String),
        #[error("{0}")]
        #[error_info(code="RE", app_code="500")]
        RespError(#[from] RespError),
        }
        "#;
        let parsed = syn::parse_str(input).unwrap();
        let info = ErrorData::from_derive_input(&parsed).unwrap();

        assert_eq!(info.ident.to_string(), "MyError");
        assert_eq!(info.prefix, "01");

        let code = process_error_info(parsed);
        println!("{}", code);
    }
}
