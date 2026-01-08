use crate::abi::{ContractAbi, EventAbi, EventParam, FunctionAbi, FunctionParam, ParamType};
use std::fs;
use std::path::Path;

/// ABI Generator that analyzes Rust source code to extract function and event definitions
pub struct AbiGenerator {
    source_code: String,
    abi: ContractAbi,
}

impl AbiGenerator {
    /// Create a new ABI generator from source code
    pub fn new(source_code: String) -> Self {
        Self {
            source_code,
            abi: ContractAbi::new(),
        }
    }

    /// Generate ABI from source code
    pub fn generate(&mut self) -> ContractAbi {
        self.extract_events();
        self.extract_functions();
        self.abi.clone()
    }

    /// Extract event definitions from source code
    fn extract_events(&mut self) {
        // Look for event! macro invocations
        let lines: Vec<&str> = self.source_code.lines().collect();

        let mut i = 0;
        while i < lines.len() {
            let line = lines[i].trim();
            if line.starts_with("event!(") {
                if let Some((event, consumed_lines)) = self.parse_event_macro_multi_line(&lines, i)
                {
                    self.abi.add_event(event);
                    i += consumed_lines;
                } else {
                    i += 1;
                }
            } else {
                i += 1;
            }
        }
    }

    /// Parse an event! macro invocation that may span multiple lines
    fn parse_event_macro_multi_line(
        &self,
        lines: &[&str],
        start_line: usize,
    ) -> Option<(EventAbi, usize)> {
        let mut event_content = String::new();
        let mut brace_count = 0;
        let mut in_event = false;
        let mut consumed_lines = 0;

        for line in lines.iter().skip(start_line) {
            let line = line.trim();
            consumed_lines += 1;

            if !in_event {
                if line.starts_with("event!(") {
                    in_event = true;
                    event_content.push_str(line);
                    // Count opening braces
                    brace_count += line.chars().filter(|&c| c == '{').count();
                    brace_count -= line.chars().filter(|&c| c == '}').count();
                }
            } else {
                event_content.push_str(line);
                // Count braces
                brace_count += line.chars().filter(|&c| c == '{').count();
                brace_count -= line.chars().filter(|&c| c == '}').count();

                // If we've closed all braces, we're done
                if brace_count == 0 && line.ends_with(");") {
                    break;
                }
            }
        }

        if !in_event {
            return None;
        }

        self.parse_event_macro(&event_content)
            .map(|event| (event, consumed_lines))
    }

    /// Parse an event! macro invocation
    fn parse_event_macro(&self, line: &str) -> Option<EventAbi> {
        // Simple parsing for event! macro
        // Example: event!(Minted { caller => Address, amount => u32, });

        let trimmed = line.trim();
        if !trimmed.starts_with("event!(") || !trimmed.ends_with(");") {
            return None;
        }

        // Extract the content between event!( and );
        let content = &trimmed[7..trimmed.len() - 2];

        // Find the event name (first word before {)
        let brace_pos = content.find('{')?;
        let event_name = content[..brace_pos].trim();

        // Extract field definitions
        let fields_content = &content[brace_pos + 1..];
        let fields_content = fields_content.trim_end_matches('}');

        let mut inputs = Vec::new();
        let field_lines: Vec<&str> = fields_content.split(',').collect();

        for field_line in field_lines {
            let field_line = field_line.trim();
            if field_line.is_empty() {
                continue;
            }

            // Parse field: name => type
            if let Some((name, param_type)) = self.parse_event_field(field_line) {
                inputs.push(EventParam {
                    name: name.to_string(),
                    kind: param_type,
                    indexed: false, // Default to false for now
                });
            }
        }

        Some(EventAbi {
            name: event_name.to_string(),
            inputs,
        })
    }

    /// Parse an event field definition
    fn parse_event_field<'a>(&self, field: &'a str) -> Option<(&'a str, ParamType)> {
        // Format: field_name => type
        let parts: Vec<&str> = field.split("=>").collect();
        if parts.len() != 2 {
            return None;
        }

        let name = parts[0].trim();
        let type_str = parts[1].trim();

        let param_type = self.parse_param_type(type_str)?;
        Some((name, param_type))
    }

    /// Parse a parameter type string
    fn parse_param_type(&self, type_str: &str) -> Option<ParamType> {
        match type_str {
            "Address" => Some(ParamType::Address),
            "u32" => Some(ParamType::Uint(32)),
            "u64" => Some(ParamType::Uint(64)),
            "u8" => Some(ParamType::Uint(8)),
            "bool" => Some(ParamType::Bool),
            "String" => Some(ParamType::String),
            "&[u8]" | "[u8]" => Some(ParamType::Bytes),
            _ => None,
        }
    }

    /// Extract function definitions from source code
    fn extract_functions(&mut self) {
        // Look for router patterns and function selectors
        let lines: Vec<&str> = self.source_code.lines().collect();

        let functions = self.collect_functions_from_router(&lines);

        // Add all collected functions
        for function in functions {
            self.abi.add_function(function);
        }
    }

    /// Collect functions from router match patterns
    fn collect_functions_from_router(&self, lines: &[&str]) -> Vec<FunctionAbi> {
        let mut functions = vec![];

        for (i, line) in lines.iter().enumerate() {
            let line = line.trim();

            // Look for selector patterns like "0x01 => {"
            if line.starts_with("0x") && line.contains("=>") {
                if let Some(selector) = self.parse_selector_from_line(line) {
                    // Check if this line has a direct function call (e.g., "0x01 => compare(call.args),")
                    if let Some(function_name) = self.extract_direct_function_call(line) {
                        if let Some(function) =
                            self.find_function_definition(lines, i, &function_name)
                        {
                            let mut function = function;
                            function.selector = selector;
                            functions.push(function);
                        }
                        continue;
                    }

                    // Check for multi-line pattern with braces
                    if line.contains('{') {
                        // Look for function calls in subsequent lines within braces
                        let mut brace_count = 0;
                        let mut in_braces = false;

                        for j in i..lines.len() {
                            let next_line = lines[j].trim();

                            // Count braces to track scope
                            for ch in next_line.chars() {
                                match ch {
                                    '{' => {
                                        brace_count += 1;
                                        in_braces = true;
                                    }
                                    '}' => {
                                        brace_count -= 1;
                                        if brace_count == 0 {
                                            in_braces = false;
                                        }
                                    }
                                    _ => {}
                                }
                            }

                            if in_braces && brace_count > 0 {
                                // Look for function calls in this line
                                if let Some(function_name) =
                                    self.extract_function_call_from_line(next_line)
                                {
                                    if let Some(function) =
                                        self.find_function_definition(lines, j, &function_name)
                                    {
                                        let mut function = function;
                                        function.selector = selector;
                                        functions.push(function);
                                    }
                                }
                            }

                            if !in_braces && brace_count == 0 {
                                break;
                            }
                        }
                    }
                }
            }
        }

        functions
    }

    /// Find function definition and create FunctionAbi
    fn find_function_definition(
        &self,
        lines: &[&str],
        start_line: usize,
        function_name: &str,
    ) -> Option<FunctionAbi> {
        // Look for function definition pattern: fn function_name(...)
        let forward = start_line..lines.len();
        let backward = 0..start_line;
        for i in forward.chain(backward) {
            let trimmed = lines[i].trim();
            if trimmed.starts_with("fn ") && trimmed.contains(function_name) {
                // Extract function signature
                if let Some(signature) = self.extract_function_signature(lines, i) {
                    return Some(self.parse_function_signature(function_name, signature));
                }
            }
        }

        None
    }

    pub fn extract_function_signature(&self, lines: &[&str], start_line: usize) -> Option<String> {
        let mut signature = String::new();

        for line in lines.iter().skip(start_line) {
            let line = line.trim();

            if line.starts_with("fn ") {
                // Start collecting the signature
                signature.push_str(line);

                // Check if this line contains the opening brace
                if line.contains('{') {
                    // Remove everything from the opening brace onwards
                    if let Some(brace_pos) = line.find('{') {
                        signature = line[..brace_pos].trim().to_string();
                    }
                    return Some(signature);
                }

                // Continue to next line if no opening brace
                continue;
            }

            // If we're already collecting a signature, add this line
            if !signature.is_empty() {
                signature.push_str(line);

                // Check if this line contains the opening brace
                if line.contains('{') {
                    // Remove everything from the opening brace onwards
                    if let Some(brace_pos) = line.find('{') {
                        signature = signature[..signature.len() - (line.len() - brace_pos)]
                            .trim()
                            .to_string();
                    }
                    return Some(signature);
                }
            }
        }

        None
    }

    pub fn parse_function_signature(&self, function_name: &str, signature: String) -> FunctionAbi {
        let mut inputs = vec![];
        let mut outputs = vec![]; // Start with empty outputs for void functions

        // Parse function signature like: fn init(caller: Address, args: &[u8])
        if let Some(params_start) = signature.find('(') {
            if let Some(params_end) = signature.find(')') {
                let params_str = &signature[params_start + 1..params_end];

                // Parse parameters
                for param in params_str.split(',') {
                    let param = param.trim();
                    if !param.is_empty() {
                        if let Some((name, param_type)) = self.parse_parameter(param) {
                            // The VM passes `caller: Address` implicitly; keep it out of the ABI so generated
                            // clients encode only the real, routed arguments.
                            if name == "caller" && param_type == ParamType::Address {
                                continue;
                            }
                            inputs.push(FunctionParam {
                                name: name.to_string(),
                                kind: param_type,
                            });
                        }
                    }
                }
            }
        }

        // Parse return type if present
        if let Some(arrow_pos) = signature.find("->") {
            let after_arrow = &signature[arrow_pos + 2..].trim();
            // Find the end of the return type (first space or brace)
            let return_type_end = after_arrow
                .find(' ')
                .unwrap_or(after_arrow.find('{').unwrap_or(after_arrow.len()));
            let return_type_str = &after_arrow[..return_type_end];
            if let Some(param_type) = self.parse_param_type_from_str(return_type_str) {
                outputs = vec![param_type];
            }
        }

        FunctionAbi {
            name: function_name.to_string(),
            selector: 0, // Will be set by caller
            inputs,
            outputs,
        }
    }

    fn parse_parameter(&self, param_str: &str) -> Option<(String, ParamType)> {
        // Parse parameter like: "caller: Address" or "args: &[u8]"
        if let Some(colon_pos) = param_str.find(':') {
            let name = param_str[..colon_pos].trim();
            let type_str = param_str[colon_pos + 1..].trim();

            if let Some(param_type) = self.parse_param_type_from_str(type_str) {
                return Some((name.to_string(), param_type));
            }
        }
        None
    }

    pub fn parse_param_type_from_str(&self, type_str: &str) -> Option<ParamType> {
        match type_str.trim() {
            "Address" => Some(ParamType::Address),
            "u8" => Some(ParamType::Uint(8)),
            "u16" => Some(ParamType::Uint(16)),
            "u32" => Some(ParamType::Uint(32)),
            "u64" => Some(ParamType::Uint(64)),
            "u128" => Some(ParamType::Uint(128)),
            "bool" => Some(ParamType::Bool),
            "&[u8]" | "[u8]" => Some(ParamType::Bytes),
            "String" | "&str" => Some(ParamType::String),
            "Result" => Some(ParamType::Result), // Proper Result type
            _ => None,
        }
    }

    fn parse_selector_from_line(&self, line: &str) -> Option<u8> {
        let parts: Vec<&str> = line.split("=>").collect();
        if parts.len() != 2 {
            return None;
        }

        let selector_str = parts[0].trim();
        if !selector_str.starts_with("0x") {
            return None;
        }

        u8::from_str_radix(&selector_str[2..], 16).ok()
    }

    fn extract_direct_function_call(&self, line: &str) -> Option<String> {
        // Look for pattern like "0x01 => compare(call.args),"
        if let Some(arrow_pos) = line.find("=>") {
            let after_arrow = &line[arrow_pos + 2..].trim();
            if let Some(start) = after_arrow.find('(') {
                let function_name = after_arrow[..start].trim();
                if !function_name.is_empty()
                    && !function_name.contains(' ')
                    && !function_name.contains("::")
                    && !function_name.contains('.')
                {
                    return Some(function_name.to_string());
                }
            }
        }
        None
    }

    fn extract_function_call_from_line(&self, line: &str) -> Option<String> {
        // Look for function call pattern: function_name(...)
        if let Some(start) = line.find('(') {
            let potential_name = line[..start].trim();
            if !potential_name.is_empty()
                && !potential_name.contains(' ')
                && !potential_name.contains("::")
                && !potential_name.contains('.')
            {
                return Some(potential_name.to_string());
            }
        }

        // Also look for patterns like "let b = balance_of(call.args)"
        if line.contains("let") && line.contains('=') {
            if let Some(equals_pos) = line.find('=') {
                let after_equals = &line[equals_pos + 1..];
                if let Some(start) = after_equals.find('(') {
                    let potential_name = after_equals[..start].trim();
                    if !potential_name.is_empty()
                        && !potential_name.contains(' ')
                        && !potential_name.contains("::")
                        && !potential_name.contains('.')
                    {
                        return Some(potential_name.to_string());
                    }
                }
            }
        }

        None
    }

    /// Generate ABI from a source file
    pub fn from_file<P: AsRef<Path>>(path: P) -> std::io::Result<ContractAbi> {
        let source_code = fs::read_to_string(path)?;
        let mut generator = AbiGenerator::new(source_code);
        Ok(generator.generate())
    }

    /// Write ABI to a JSON file
    pub fn write_abi_to_file<P: AsRef<Path>>(abi: &ContractAbi, path: P) -> std::io::Result<()> {
        let json = abi.to_json();
        fs::write(path, json)
    }
}

/// Generate ABI for all example programs
pub fn generate_all_example_abis() -> std::io::Result<()> {
    let examples_dir = "crates/examples/src";
    let bin_dir = "crates/examples/bin";

    // Ensure bin directory exists
    fs::create_dir_all(bin_dir)?;

    let source_files = vec![
        "simple.rs",
        "storage.rs",
        "erc20.rs",
        "multi_func.rs",
        "call_program.rs",
    ];

    for source_file in source_files {
        let source_path = format!("{examples_dir}/{source_file}");
        let abi_path = format!("{bin_dir}/{}.abi.json", source_file.replace(".rs", ""));

        println!("Generating ABI for {source_file}");

        match AbiGenerator::from_file(&source_path) {
            Ok(abi) => {
                AbiGenerator::write_abi_to_file(&abi, &abi_path)?;
                println!("  ✓ Generated {abi_path}");
            }
            Err(e) => {
                eprintln!("  ✗ Failed to generate ABI for {source_file}: {e}");
            }
        }
    }

    Ok(())
}
