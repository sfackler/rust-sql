use linked_hash_map::LinkedHashMap;
use std::fs::File;
use std::io::{BufWriter, Write};

const ERRCODES_TXT: &str = include_str!("errcodes.txt");

pub fn build() {
    let mut file = BufWriter::new(File::create("../tokio-postgres/src/error/sqlstate.rs").unwrap());

    let codes = parse_codes();

    make_type(&mut file);
    make_code(&codes, &mut file);
    make_consts(&codes, &mut file);
    make_inner(&codes, &mut file);
    make_map(&codes, &mut file);
}

fn parse_codes() -> LinkedHashMap<String, Vec<String>> {
    let mut codes = LinkedHashMap::new();

    for line in ERRCODES_TXT.lines() {
        if line.starts_with('#') || line.starts_with("Section") || line.trim().is_empty() {
            continue;
        }

        let mut it = line.split_whitespace();
        let code = it.next().unwrap().to_owned();
        it.next();
        let name = it.next().unwrap().replace("ERRCODE_", "");

        codes.entry(code).or_insert_with(Vec::new).push(name);
    }

    codes
}

fn make_type(file: &mut BufWriter<File>) {
    write!(
        file,
        "// Autogenerated file - DO NOT EDIT

/// A SQLSTATE error code
#[derive(PartialEq, Eq, Clone, Debug)]
pub struct SqlState(Inner);

impl SqlState {{
    /// Creates a `SqlState` from its error code.
    pub fn from_code(s: &str) -> SqlState {{
        match SQLSTATE_MAP.get(s) {{
            Some(state) => state.clone(),
            None => SqlState(Inner::Other(s.into())),
        }}
    }}
"
    )
    .unwrap();
}

fn make_code(codes: &LinkedHashMap<String, Vec<String>>, file: &mut BufWriter<File>) {
    write!(
        file,
        r#"
    /// Returns the error code corresponding to the `SqlState`.
    pub fn code(&self) -> &str {{
        match &self.0 {{"#,
    )
    .unwrap();

    for code in codes.keys() {
        write!(
            file,
            r#"
            Inner::E{code} => "{code}","#,
            code = code,
        )
        .unwrap();
    }

    write!(
        file,
        r#"
            Inner::Other(code) => code,
        }}
    }}
        "#
    )
    .unwrap();
}

fn make_consts(codes: &LinkedHashMap<String, Vec<String>>, file: &mut BufWriter<File>) {
    for (code, names) in codes {
        for name in names {
            write!(
                file,
                r#"
    /// {code}
    pub const {name}: SqlState = SqlState(Inner::E{code});
"#,
                name = name,
                code = code,
            )
            .unwrap();
        }
    }

    write!(file, "}}").unwrap();
}

fn make_inner(codes: &LinkedHashMap<String, Vec<String>>, file: &mut BufWriter<File>) {
    write!(
        file,
        r#"

#[derive(PartialEq, Eq, Clone, Debug)]
#[allow(clippy::upper_case_acronyms)]
enum Inner {{"#,
    )
    .unwrap();
    for code in codes.keys() {
        write!(
            file,
            r#"
    E{},"#,
            code,
        )
        .unwrap();
    }
    write!(
        file,
        r#"
    Other(Box<str>),
}}
        "#,
    )
    .unwrap();
}

fn make_map(codes: &LinkedHashMap<String, Vec<String>>, file: &mut BufWriter<File>) {
    let mut builder = phf_codegen::Map::new();
    for (code, names) in codes {
        builder.entry(&**code, &format!("SqlState::{}", &names[0]));
    }
    write!(
        file,
        "
#[rustfmt::skip]
static SQLSTATE_MAP: phf::Map<&'static str, SqlState> = \n{};\n",
        builder.build()
    )
    .unwrap();
}
