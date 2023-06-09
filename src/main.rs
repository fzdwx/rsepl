use std::{
    ffi::CString,
    io::{self, Write},
};

fn main() {
    let mut fn_extern = String::from("extern \"C\" {\n");
    loop {
        print!("rsepl> ");
        io::stdout().flush().unwrap();
        let mut buf = String::new();
        if let Ok(_input) = io::stdin().read_line(&mut buf) {
            let input = buf.trim();
            if input.starts_with("fn") {
                do_compile_to_so(input);
                let fn_sign = parse_fn_signature(input);
                fn_extern.push_str(fn_sign.as_str());
            } else {
                do_eval(input, fn_extern.as_str());
            }
        }
    }
}

fn do_eval(input: &str, fn_extern: &str) {
    let (input, fn_name) = wrapper(input,fn_extern);
    if let Some(v) = compile(input.as_str()) {
        type MyFn = fn() -> i32;
        if let Some(f) = get_symbol(v, fn_name.as_str()) {
            let f: MyFn = unsafe { std::mem::transmute(f) };
            println!("result: {}", f());
        }
    } else {
        println!("compile failed")
    }
}

fn wrapper(input: &str,fn_extern:&str) -> (String, String) {
    // get randme name
    let r = unsafe { libc::rand() };
    let randme_fn_name = format!("wrapper_{}", r);

    (
        format!(
            "fn {}() -> i32 {{
   unsafe{{
    return {}
   }}
}}

{}
}}

",
            randme_fn_name, input,fn_extern
        ),
        randme_fn_name,
    )
}

fn do_compile_to_so(input: &str) {
    if let Some(_v) = compile(input) {
        println!("compile success")
    } else {
        println!("compile failed")
    }
}

fn compile(input: &str) -> Option<*mut libc::c_void> {
    let text = to_rs_file(input);
    if let Ok(f) = tempfile::Builder::new()
        .prefix("asd")
        .suffix(".rs")
        .tempfile()
    {
        let path = f.path().to_str().unwrap().replace("asd", "rspel_asd_");
        let path = path.as_str();
        let mut file = std::fs::File::create(path).unwrap();
        file.write_all(text.as_bytes()).unwrap();
        file.flush().unwrap();

        let output = std::process::Command::new("rustc")
            .arg(path)
            .arg("--crate-type=cdylib")
            .arg("--out-dir=/tmp")
            .output()
            .expect("failed to execute process");
        if output.status.success() {
            return Some(load_lib(to_so_path(path).as_str()));
        }else{
            println!("compile failed: {}",String::from_utf8(output.stderr).unwrap());
        }
    }
    None
}

fn parse_fn_signature(text: &str) -> String {
    let mut fn_signature = String::new();
    for c in text.chars() {
        if c == '{' {
            break;
        } else {
            fn_signature.push(c);
        }
    }
    fn_signature.push(';');
    fn_signature
}

fn to_so_path(path: &str) -> String {
    path.replace(".rs", ".so").replace("/tmp/", "/tmp/lib")
}

fn to_rs_file(input: &str) -> String {
    format!("#[no_mangle]\n pub {}", input)
}

fn load_lib(filename: &str) -> *mut libc::c_void {
    unsafe {
        let filename = CString::new(filename).unwrap();
        let handle = libc::dlopen(filename.as_ptr(), libc::RTLD_NOW | libc::RTLD_GLOBAL);
        if handle.is_null() {
            let error = std::ffi::CStr::from_ptr(libc::dlerror());
            panic!("load_lib failed: {:?}", error);
        }
        handle
    }
}

fn get_symbol(handle: *mut libc::c_void, symbol: &str) -> Option<*mut libc::c_void> {
    unsafe {
        let symbol = CString::new(symbol).unwrap();
        let symbol = libc::dlsym(handle, symbol.as_ptr());
        if symbol.is_null() {
            return None;
        }
        Some(symbol)
    }
}
