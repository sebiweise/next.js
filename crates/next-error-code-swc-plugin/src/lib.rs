use std::{
    collections::{hash_map::DefaultHasher, HashMap},
    fs,
    hash::{Hash, Hasher},
};

use swc_core::{
    ecma::{ast::*, transforms::testing::test_inline, visit::*},
    plugin::{plugin_transform, proxies::TransformPluginProgramMetadata},
};

pub struct TransformVisitor {
    commit_hash: String,
    file_path: String,
    mode: String,
    string_occurrences: HashMap<String, usize>,
}

impl TransformVisitor {
    // Get the string representation of the first argument of `new Error(...)`
    fn stringify_new_error_arg(&self, expr: &Expr) -> String {
        match expr {
            Expr::Lit(lit) => match lit {
                Lit::Str(str_lit) => str_lit.value.to_string(),
                _ => "%s".to_string(),
            },

            Expr::Tpl(tpl) => {
                let mut result = String::new();
                let mut expr_iter = tpl.exprs.iter();

                for (_i, quasi) in tpl.quasis.iter().enumerate() {
                    result.push_str(&quasi.raw);
                    if let Some(expr) = expr_iter.next() {
                        result.push_str(&self.stringify_new_error_arg(expr));
                    }
                }
                result
            }

            Expr::Bin(bin_expr) => {
                // Assume binary expression is always add for two strings
                format!(
                    "{}{}",
                    self.stringify_new_error_arg(&bin_expr.left),
                    self.stringify_new_error_arg(&bin_expr.right)
                )
            }

            _ => "%s".to_string(),
        }
    }

    // Get error code, which is computed based on
    // 1. Git commit hash, concatenated with
    // 2. the hash of object { file_path, error_message, occurrence_count }
    fn get_error_code(&mut self, first_arg: &ExprOrSpread) -> String {
        let error_message: String = self.stringify_new_error_arg(&first_arg.expr);

        // Maintain string_occurrences
        *self
            .string_occurrences
            .entry(error_message.clone())
            .or_insert(0) += 1;

        // Generate the hash of object { file_path, error_message, occurrence_count }
        let mut hasher = DefaultHasher::new();
        serde_json::json!({
            "file_path": self.file_path,
            "error_message": error_message,
            "occurrence_count": self.string_occurrences[&error_message]
        })
        .to_string()
        .hash(&mut hasher);
        let hash = format!("{:08x}", hasher.finish());

        // Concatenate the commit hash and the hash
        let error_code = format!("E{}{}", self.commit_hash, hash);

        // Save to JSON file
        let error_metadata = serde_json::json!({
            "file_path": self.file_path,
            "error_message": error_message,
            "occurrence_count": self.string_occurrences[&error_message]
        })
        .to_string();
        let mut retries = 3;

        // Do not do FS operations in unit tests
        if cfg!(test) {
            return error_code;
        }

        loop {
            if self.mode == "check" {
                let error_code_path = format!("cwd/error_codes/{}.json", hash);
                if !std::path::Path::new(&error_code_path).exists() {
                    panic!(
                        "ERROR: File {} does not exist.\n\nREQUIRED ACTION:\n1. Run `pnpm \
                         build`\n2. Commit all file changes from \
                         /packages/next/error-codes\n\nThis is required to maintain error code \
                         consistency.",
                        format!("/packages/next/error_codes/{}.json", hash)
                    );
                }
                return error_code;
            }

            if self.mode != "generate" {
                panic!("Mode must be 'generate', got '{}'", self.mode);
            }

            fs::create_dir_all("cwd/error_codes").unwrap_or_else(|e| {
                panic!("Failed to create error_codes directory: {}", e);
            });

            match fs::write(format!("cwd/error_codes/{}.json", hash), &error_metadata) {
                Ok(_) => break,
                Err(e) => {
                    retries -= 1;
                    if retries == 0 {
                        panic!("Failed to write error metadata after 3 retries: {}", e);
                    }
                }
            }
        }

        error_code
    }
}

impl VisitMut for TransformVisitor {
    fn visit_mut_expr(&mut self, expr: &mut Expr) {
        expr.visit_mut_children_with(self);

        let mut code: Option<String> = None;
        let mut new_error_expr: Option<&NewExpr> = None;

        if let Expr::New(new_expr) = expr {
            if let Expr::Ident(ident) = &*new_expr.callee {
                if ident.sym.to_string() == "Error" {
                    new_error_expr = Some(new_expr);

                    if let Some(args) = &new_expr.args {
                        if let Some(first_arg) = args.first() {
                            code = Some(self.get_error_code(&first_arg));
                        }
                    }
                }
            }
        }

        if let Some(code) = code {
            if let Some(new_error_expr) = new_error_expr {
                *expr = Expr::Call(CallExpr {
                    span: new_error_expr.span,
                    callee: Callee::Expr(Box::new(Expr::Member(MemberExpr {
                        span: new_error_expr.span,
                        obj: Box::new(Expr::Ident(Ident::new(
                            "Object".into(),
                            new_error_expr.span,
                            Default::default(),
                        ))),
                        prop: MemberProp::Ident("assign".into()),
                    }))),
                    args: vec![
                        ExprOrSpread {
                            spread: None,
                            expr: Box::new(Expr::New(new_error_expr.clone())),
                        },
                        ExprOrSpread {
                            spread: None,
                            expr: Box::new(Expr::Object(ObjectLit {
                                span: new_error_expr.span,
                                props: vec![PropOrSpread::Prop(Box::new(Prop::KeyValue(
                                    KeyValueProp {
                                        key: PropName::Ident("__NEXT_ERROR_CODE".into()),
                                        value: Box::new(Expr::Lit(Lit::Str(Str {
                                            span: new_error_expr.span,
                                            value: code.into(),
                                            raw: None,
                                        }))),
                                    },
                                )))],
                            })),
                        },
                    ],
                    type_args: None,
                    ctxt: new_error_expr.ctxt,
                });
            }
        }
    }
}

#[plugin_transform]
pub fn process_transform(
    mut program: Program,
    metadata: TransformPluginProgramMetadata,
) -> Program {
    fn parse_config(metadata: TransformPluginProgramMetadata) -> (String, String, String) {
        let config_str = metadata
            .get_transform_plugin_config()
            .unwrap_or_else(|| panic!("Failed to get transform plugin config"))
            .to_string();

        let config: serde_json::Value = serde_json::from_str(&config_str)
            .unwrap_or_else(|e| panic!("failed to parse config: {}, config: {}", e, config_str));

        (
            config["commitHash"]
                .as_str()
                .unwrap_or_else(|| panic!("commitHash not found in config: {}", config_str))
                .to_string(),
            config["filePath"]
                .as_str()
                .unwrap_or_else(|| panic!("filePath not found in config: {}", config_str))
                .to_string(),
            config["mode"]
                .as_str()
                .unwrap_or_else(|| panic!("mode not found in config: {}", config_str))
                .to_string(),
        )
    }

    let (commit_hash, file_path, mode) = parse_config(metadata);
    let string_occurrences: HashMap<String, usize> = HashMap::new();

    let mut visitor = TransformVisitor {
        commit_hash,
        file_path,
        mode,
        string_occurrences,
    };

    visitor.visit_mut_program(&mut program);
    program
}

test_inline!(
    Default::default(),
    |_| visit_mut_pass(TransformVisitor {
        commit_hash: "0000000000".to_string(),
        file_path: "/test/file.js".to_string(),
        string_occurrences: HashMap::new(),
        mode: "compile".to_string(),
    }),
    realistic_api_handler,
    // Input codes
    r#"
async function fetchUserData(userId) {
    try {
        const response = await fetch(`/api/users/${userId}`);
        if (!response.ok) {
            throw new Error(`Failed to fetch user ${userId}: ${response.statusText}`);
        }
        return await response.json();
    } catch (err) {
        throw new Error(`Request failed: ${err.message}`);
    }
}"#,
    // Output codes after transformed with plugin
    r#"
async function fetchUserData(userId) {
    try {
        const response = await fetch(`/api/users/${userId}`);
        if (!response.ok) {
            throw Object.assign(new Error(`Failed to fetch user ${userId}: ${response.statusText}`), { __NEXT_ERROR_CODE: "E000000000026c63d53d605f848" });
        }
        return await response.json();
    } catch (err) {
        throw Object.assign(new Error(`Request failed: ${err.message}`), { __NEXT_ERROR_CODE: "E0000000000a5151f4ce82c5c79" });
    }
}"#
);
