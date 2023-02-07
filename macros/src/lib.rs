use std::str::FromStr;
use proc_macro::TokenStream;
use quote::{format_ident, quote, ToTokens};
use syn::punctuated::Punctuated;
use syn::spanned::Spanned;
use syn::{ItemFn, FnArg, AttributeArgs, parse_macro_input, NestedMeta, Meta, Token, Pat, Type};
use syn;
use syn::Stmt;



#[proc_macro_attribute]
pub fn function(
    _args: TokenStream,
    item: TokenStream,
) -> TokenStream {
    //let attrs = parse_macro_input!(args as AttributeArgs);
    let mut func = parse_macro_input!(item as ItemFn);
    //let register_function_name;
    //let destructure_to_struct;
    /*
    match &attrs[0] {
        NestedMeta::Meta(meta) => {
            match meta {
                Meta::Path(p) => {
                    let seg = &p.segments;
                    let path_segments = seg.first().unwrap();
                    register_function_name = path_segments.ident.clone();
                }
                _ => panic!()
            }
        }
        _ => panic!()
    }
    match &attrs[1] {
        NestedMeta::Meta(meta) => {
            match meta {
                Meta::Path(p) => {
                    let seg = &p.segments;
                    let path_segments = seg.first().unwrap();
                    destructure_to_struct = path_segments.ident.clone();
                }
                _ => panic!()
            }
        }
        _ => panic!()
    }


    
    */
    //dbg!(&attrs);
    //dbg!(&attrs[0]);
    //dbg!(&func);
    let inputs = func.sig.inputs.clone();
    let mut ps: Punctuated<FnArg, Token!(,)> = Punctuated::new();
    let length = inputs.len();
    let mut s_v = vec![];
    if length != 0 {

    
    

    let to_put_at_args: FnArg = syn::parse(TokenStream::from_str("input: String").unwrap()).unwrap();
    ps.push(to_put_at_args);
    func.sig.inputs = ps;
    let mut s = String::from("let (");
    let mut s2 = String::from(": (");
    for arg in inputs.iter() {
        let id = match arg {
            FnArg::Typed(t) => {
                match *(t.pat.clone()) {
                    Pat::Ident(ident) => {
                        ident.ident
                    }
                    _ => panic!()
                }
                }
            _ => panic!()
        };
        let ident = id.to_string();
        s.push_str(&ident);
        s.push_str(", ");
        s_v.push(ident);
        let id = match arg {
            FnArg::Typed(t) => {
                match *(t.ty.clone()) {
                    Type::Path(ident) => {
                        let i = ident.path.segments;
                        let i = i.first().unwrap();
                        i.ident.clone()
                    }
                    _ => panic!()
                }
                }
            _ => panic!()
        };
        let ident = id.to_string();
        s2.push_str(&ident);
        s2.push_str(", ");
    }
    s.push_str(")");
    s2.push_str(") = ::dolphine::serde_json::from_str(input).unwrap();"); // change unwrap to result
    s.push_str(&s2);
    // dbg!(&s);

    if !s_v.contains(&"input".to_string()) {
        let stmt: Stmt = syn::parse(TokenStream::from_str("drop(input);").unwrap()).unwrap();
        func.block.stmts.insert(0, stmt);
    }

    println!("{}", s);
    // add s to fn here
    let stmt: Stmt = syn::parse(TokenStream::from_str(&s).unwrap()).unwrap();
    func.block.stmts.insert(0, stmt);
    }
    //println!("{:?}", func.sig.inputs.into_token_stream());
    //dbg!("Got here");
    let q = quote!(#func);
    return q.into();
    }
