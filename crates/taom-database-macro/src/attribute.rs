use syn::{Attribute, LitStr, Result, Token};

macro_rules! fail {
    ($t:expr, $m:expr) => {
        return Err(syn::Error::new_spanned($t, $m))
    };
}

macro_rules! try_set {
    (option : $i:expr, $v:expr, $t:expr) => {
        match $i {
            None => { $i = Some($v); Ok(()) },
            Some(_) => fail!($t, "duplicate attribute"),
        }
    };
    (bool : $i:expr, $t:expr) => {
        match $i {
            false => { $i = true; Ok(()) },
            true => fail!($t, "duplicate attribute"),
        }
    };
}

#[derive(Default)]
pub(crate) struct DbRowAttribute {
    pub rename: Option<String>,
    pub default: bool,
}

pub fn parse_db_row_attr(attrs: &[Attribute]) -> Result<DbRowAttribute> {
    let mut result_attr = DbRowAttribute::default();

    for attr in  attrs.iter().filter(|attr| attr.path().is_ident("db_row")) {
        attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("rename") {
                meta.input.parse::<Token![=]>()?;
                let val: LitStr = meta.input.parse()?;
                try_set!(option : result_attr.rename, val.value(), val)
            } else if meta.path.is_ident("default") {
                try_set!(bool : result_attr.default, meta.path)
            } else {
                let msg = match meta.path.get_ident() {
                    Some(ident) => format!("Unexpected attribute `{}` in db_row", ident),
                    None => "Unexpected attribute".to_string()
                };
                fail!(meta.path, msg)
            }
        })?
    }

    Ok(result_attr)
}
