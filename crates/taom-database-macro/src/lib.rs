use attribute::parse_db_row_attr;
use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, DeriveInput, Fields, Result};

mod attribute;

#[proc_macro_derive(FromRow, attributes(db_row))]
pub fn derive_from_row(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    let syn::Data::Struct(data) = input.data else {
        return TokenStream::from(
            syn::Error::new(input.ident.span(), "Only structs can derive `FromRow`")
                .to_compile_error(),
        );
    };

    let struct_name = input.ident;
    let fields = data
        .fields
        .iter()
        .enumerate()
        .map(|(i, field)| {
            Ok(match field.ident.as_ref() {
                Some(name) => {
                    let attr = parse_db_row_attr(field.attrs.as_slice())?;
                    let db_name = attr.rename.unwrap_or(name.to_string());
                    quote! { #name: row.try_get(#db_name)? }
                }
                None => quote! { row.try_get(#i)? },
            })
        })
        .collect::<Result<Vec<_>>>();

    let fields = match fields {
        Ok(ts) => ts,
        Err(e) => return e.to_compile_error().into(),
    };

    let struct_self = match data.fields {
        Fields::Named(_) => quote! { Self {#(#fields),*} },
        Fields::Unnamed(_) => quote! { Self(#(#fields),*) },
        Fields::Unit => quote! { Self },
    };

    quote! {
        #[automatically_derived]
        impl ::taom_database::FromRow for #struct_name {
            fn from_row(row: ::tokio_postgres::Row) -> Result<Self, ::tokio_postgres::Error> {
                Ok(#struct_self)
            }
        }
    }
    .into()
}
