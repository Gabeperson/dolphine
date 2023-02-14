use proc_macro::TokenStream;
use quote::quote;
use std::str::FromStr;
use syn;
use syn::punctuated::Punctuated;
use syn::Block;
use syn::ReturnType;
use syn::Stmt;
use syn::{parse_macro_input, FnArg, ItemFn, Pat, Token, Type};

#[proc_macro_attribute]
pub fn function(_args: TokenStream, item: TokenStream) -> TokenStream {
    let mut func = parse_macro_input!(item as ItemFn);
    if func.sig.asyncness.is_some() {
        panic!(
            "Function macro cannot be used on synchronous function! Use 'async_function' instead!"
        );
    }
    let inputs = func.sig.inputs.clone();
    let mut ps: Punctuated<FnArg, Token!(,)> = Punctuated::new();
    let length = inputs.len();
    let mut s_v = vec![];
    if length != 0 {
        let to_put_at_args: FnArg =
            syn::parse(TokenStream::from_str("input: String").unwrap()).unwrap();
        ps.push(to_put_at_args);
        func.sig.inputs = ps;
        let mut s = String::from("let (");
        let mut s2 = String::from(": (");
        for arg in inputs.iter() {
            let id = match arg {
                FnArg::Typed(t) => match *(t.pat.clone()) {
                    Pat::Ident(ident) => ident.ident,
                    _ => panic!(),
                },
                _ => panic!(),
            };
            let ident = id.to_string();
            s.push_str(&ident);
            s.push_str(", ");
            s_v.push(ident);
            let id = match arg {
                FnArg::Typed(t) => match *(t.ty.clone()) {
                    Type::Path(ident) => {
                        let i = ident.path.segments;
                        let i = i.first().unwrap();
                        i.ident.clone()
                    }
                    _ => panic!(),
                },
                _ => panic!(),
            };
            let ident = id.to_string();
            s2.push_str(&ident);
            s2.push_str(", ");
        }
        s.push_str(")");
        s2.push_str(") = ::dolphine::serde_json::from_str(&input)?;"); // change unwrap to result
        s.push_str(&s2);
        if !s_v.contains(&"input".to_string()) {
            let stmt: Stmt = syn::parse(TokenStream::from_str("drop(input);").unwrap()).unwrap();
            func.block.stmts.insert(0, stmt);
        }
        let stmt: Stmt = syn::parse(TokenStream::from_str(&s).unwrap()).unwrap();
        func.block.stmts.insert(0, stmt);

        // wrapper function shenanigans
        let return_type = func.sig.output.clone();
        let return_type = quote!(#return_type);
        let return_type_outer: ReturnType =
            syn::parse(TokenStream::from_str("-> Result<String, ::dolphine::Report>").unwrap())
                .unwrap();
        func.sig.output = return_type_outer;
        let block = func.block.clone();
        let block = quote!(#block).to_string();
        let block = block.trim().to_string();
        let mut it = block.chars();
        it.next();
        it.next_back();
        let block: String = it.collect();

        let start = format!(
            "
            let t = (|{}| {}{{
        ",
            if length != 0 {
                "input: String"
            } else {
                ""
            },
            return_type.to_string()
        );

        println!("{}", return_type.to_string());

        let end = format!("
        }})({});
        match t {{
            Ok(v) => {{
                let r = ::dolphine::serde_json::to_string(&v);
                match r {{
                    Ok(s) => return Ok(s),
                    Err(e) => return Err(Report::new(e)),
                }}
            }},
            Err(e) => return Err(e),
        }};
        ", if length != 0 {
            "input"
        } else {
            ""
        });

        let block = format!("{{{}{}{}}}", start, block, end);
        let s: Block = syn::parse(TokenStream::from_str(&block).unwrap()).unwrap();
        func.block = Box::new(s);
    }

    let q = quote!(#func);
    return q.into();
}

#[proc_macro_attribute]
pub fn async_function(_args: TokenStream, item: TokenStream) -> TokenStream {
    let mut func = parse_macro_input!(item as ItemFn);
    if func.sig.asyncness.is_none() {
        panic!("async_function macro can only be used on synchronous functions! Use 'function' instead!");
    }
    func.sig.asyncness = None;
    let inputs = func.sig.inputs.clone();
    let mut ps: Punctuated<FnArg, Token!(,)> = Punctuated::new();
    let length = inputs.len();
    let mut s_v = vec![];
    let to_put_at_args: FnArg =
            syn::parse(TokenStream::from_str("input: String").unwrap()).unwrap();
        ps.push(to_put_at_args);
        func.sig.inputs = ps;
    if length != 0 {
        let mut s = String::from("let (");
        let mut s2 = String::from(": (");
        for arg in inputs.iter() {
            let id = match arg {
                FnArg::Typed(t) => match *(t.pat.clone()) {
                    Pat::Ident(ident) => ident.ident,
                    _ => panic!(),
                },
                _ => panic!(),
            };
            let ident = id.to_string();
            s.push_str(&ident);
            s.push_str(", ");
            s_v.push(ident);
            let id = match arg {
                FnArg::Typed(t) => match *(t.ty.clone()) {
                    Type::Path(ident) => {
                        let i = ident.path.segments;
                        let i = i.first().unwrap();
                        i.ident.clone()
                    }
                    _ => panic!(),
                },
                _ => panic!(),
            };
            let ident = id.to_string();
            s2.push_str(&ident);
            s2.push_str(", ");
        }
        s.push_str(")");
        s2.push_str(") = ::dolphine::serde_json::from_str(&input)?;"); // change unwrap to result
        s.push_str(&s2);
        if !s_v.contains(&"input".to_string()) {
            let stmt: Stmt = syn::parse(TokenStream::from_str("drop(input);").unwrap()).unwrap();
            func.block.stmts.insert(0, stmt);
        }
        let stmt: Stmt = syn::parse(TokenStream::from_str(&s).unwrap()).unwrap();
        func.block.stmts.insert(0, stmt);

        
    }
    // wrapper function shenanigans
    let return_type = func.sig.output.clone();
    let return_type = quote!(#return_type);
    let return_type_outer: ReturnType =
        syn::parse(TokenStream::from_str("-> Result<String, ::dolphine::Report>").unwrap())
            .unwrap();
    func.sig.output = return_type_outer;
    let block = func.block.clone();
    let block = quote!(#block).to_string();
    let block = block.trim().to_string();
    let mut it = block.chars();
    it.next();
    it.next_back();
    let block: String = it.collect();

    let block = format!(
        "
        ::dolphine::tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap()
            .block_on(async {{
                {}
            }})",
        block
    );

    let start = format!(
        "
        let t = (|{}| {}{{
    ",
        if length != 0 {
            "input: String"
        } else {
            ""
        },
        return_type.to_string()
    );

    //println!("{}", return_type.to_string());
    let end = format!("
        }})({});
        match t {{
            Ok(v) => {{
                let r = ::dolphine::serde_json::to_string(&v);
                match r {{
                    Ok(s) => return Ok(s),
                    Err(e) => return Err(Report::new(e)),
                }}
            }},
            Err(e) => return Err(e),
        }};
    ", if length != 0 {
        "input"
    } else {
        ""
    });

    let block = format!("{{{}{}{}}}", start, block, end);
    //println!("{}", &block);
    let s: Block = syn::parse(TokenStream::from_str(&block).unwrap()).unwrap();
    func.block = Box::new(s);

    let q = quote!(#func);
    return q.into();
}
