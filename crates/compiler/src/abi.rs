#[derive(Debug, Clone)]
pub struct EventAbi {
    pub name: String,
    pub inputs: Vec<EventParam>,
}

#[derive(Debug, Clone)]
pub struct EventParam {
    pub name: String,
    pub kind: ParamType,
    pub indexed: bool,
}

impl EventAbi {
    /// Returns the event ID: first 32 bytes of the event name, padded with zeros.
    pub fn id(&self) -> [u8; 32] {
        let name_bytes = self.name.as_bytes();
        let mut id = [0u8; 32];
        let len = name_bytes.len().min(32);
        id[..len].copy_from_slice(&name_bytes[..len]);
        id
    }
}

#[derive(Debug, Clone)]
pub struct FunctionAbi {
    pub name: String,
    pub selector: u8,
    pub inputs: Vec<FunctionParam>,
    pub outputs: Vec<ParamType>,
}

#[derive(Debug, Clone)]
pub struct FunctionParam {
    pub name: String,
    pub kind: ParamType,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParamType {
    Address,
    Uint(usize), // bits
    Bool,
    Bytes,
    String,
    Result, // Represents the Result struct with success, error_code, data_len, and data fields
            // Extend as needed
}

impl ParamType {
    /// Convert to JSON-compatible string representation
    pub fn to_json_string(&self) -> String {
        match self {
            ParamType::Address => "address".to_string(),
            ParamType::Uint(bits) => format!("uint{bits}"),
            ParamType::Bool => "bool".to_string(),
            ParamType::Bytes => "bytes".to_string(),
            ParamType::String => "string".to_string(),
            ParamType::Result => "result".to_string(),
        }
    }

    /// Parse from JSON string representation
    pub fn from_json_string(s: &str) -> Self {
        match s {
            "address" => ParamType::Address,
            "bool" => ParamType::Bool,
            "bytes" => ParamType::Bytes,
            "string" => ParamType::String,
            "result" => ParamType::Result,
            s if s.starts_with("uint") => {
                let bits_str = &s[4..];
                let bits = bits_str.parse::<usize>().unwrap_or(32);
                ParamType::Uint(bits)
            }
            _ => ParamType::Bytes, // Default fallback
        }
    }
}

/// Complete ABI for a smart contract
#[derive(Debug, Clone)]
pub struct ContractAbi {
    pub version: String,
    pub functions: Vec<FunctionAbi>,
    pub events: Vec<EventAbi>,
}

impl ContractAbi {
    /// Create a new empty ABI
    pub fn new() -> Self {
        Self {
            version: "1.0".to_string(),
            functions: Vec::new(),
            events: Vec::new(),
        }
    }

    /// Add a function to the ABI
    pub fn add_function(&mut self, function: FunctionAbi) {
        self.functions.push(function);
    }

    /// Add an event to the ABI
    pub fn add_event(&mut self, event: EventAbi) {
        self.events.push(event);
    }

    /// Convert to JSON string
    pub fn to_json(&self) -> String {
        let mut json = String::new();
        json.push_str("{\n");
        json.push_str(&format!("  \"version\": \"{}\",\n", self.version));

        // Functions
        json.push_str("  \"functions\": [\n");
        for (i, func) in self.functions.iter().enumerate() {
            json.push_str("    {\n");
            json.push_str(&format!("      \"name\": \"{}\",\n", func.name));
            json.push_str(&format!("      \"selector\": {},\n", func.selector));

            // Inputs
            json.push_str("      \"inputs\": [\n");
            for (j, input) in func.inputs.iter().enumerate() {
                json.push_str("        {\n");
                json.push_str(&format!("          \"name\": \"{}\",\n", input.name));
                json.push_str(&format!(
                    "          \"type\": \"{}\"\n",
                    input.kind.to_json_string()
                ));
                if j < func.inputs.len() - 1 {
                    json.push_str("        },\n");
                } else {
                    json.push_str("        }\n");
                }
            }
            json.push_str("      ],\n");

            // Outputs
            json.push_str("      \"outputs\": [\n");
            for (j, output) in func.outputs.iter().enumerate() {
                json.push_str("        {\n");
                json.push_str(&format!(
                    "          \"type\": \"{}\"\n",
                    output.to_json_string()
                ));
                if j < func.outputs.len() - 1 {
                    json.push_str("        },\n");
                } else {
                    json.push_str("        }\n");
                }
            }
            json.push_str("      ]\n");

            if i < self.functions.len() - 1 {
                json.push_str("    },\n");
            } else {
                json.push_str("    }\n");
            }
        }
        json.push_str("  ],\n");

        // Events
        json.push_str("  \"events\": [\n");
        for (i, event) in self.events.iter().enumerate() {
            json.push_str("    {\n");
            json.push_str(&format!("      \"name\": \"{}\",\n", event.name));
            json.push_str("      \"inputs\": [\n");
            for (j, input) in event.inputs.iter().enumerate() {
                json.push_str("        {\n");
                json.push_str(&format!("          \"name\": \"{}\",\n", input.name));
                json.push_str(&format!(
                    "          \"type\": \"{}\",\n",
                    input.kind.to_json_string()
                ));
                json.push_str(&format!("          \"indexed\": {}\n", input.indexed));
                if j < event.inputs.len() - 1 {
                    json.push_str("        },\n");
                } else {
                    json.push_str("        }\n");
                }
            }
            json.push_str("      ]\n");

            if i < self.events.len() - 1 {
                json.push_str("    },\n");
            } else {
                json.push_str("    }\n");
            }
        }
        json.push_str("  ]\n");
        json.push_str("}\n");

        json
    }

    /// Parse ABI from JSON string
    pub fn from_json(json_str: &str) -> Result<Self, String> {
        // Simple JSON parser for ABI
        let mut abi = ContractAbi::new();

        // Extract version (optional, use default if not found)
        if let Some(version_start) = json_str.find("\"version\"") {
            if let Some(colon) = json_str[version_start..].find(':') {
                let after_colon = &json_str[version_start + colon + 1..];
                if let Some(quote1) = after_colon.find('"') {
                    if let Some(quote2) = after_colon[quote1 + 1..].find('"') {
                        abi.version = after_colon[quote1 + 1..quote1 + 1 + quote2].to_string();
                    }
                }
            }
        }

        // Parse functions
        if let Some(functions_start) = json_str.find("\"functions\"") {
            if let Some(array_start) = json_str[functions_start..].find('[') {
                let functions_section = &json_str[functions_start + array_start + 1..];
                if let Some(array_end) = Self::find_matching_bracket(functions_section, '[', ']') {
                    let functions_content = &functions_section[..array_end];

                    // Parse each function
                    let mut pos = 0;
                    while pos < functions_content.len() {
                        if let Some(obj_start) = functions_content[pos..].find('{') {
                            let obj_content = &functions_content[pos + obj_start + 1..];
                            if let Some(obj_end) =
                                Self::find_matching_bracket(obj_content, '{', '}')
                            {
                                let function_json = &obj_content[..obj_end];
                                if let Ok(function) = Self::parse_function(function_json) {
                                    abi.functions.push(function);
                                }
                                pos = pos + obj_start + obj_end + 2;
                            } else {
                                break;
                            }
                        } else {
                            break;
                        }
                    }
                }
            }
        }

        // Parse events
        if let Some(events_start) = json_str.find("\"events\"") {
            if let Some(array_start) = json_str[events_start..].find('[') {
                let events_section = &json_str[events_start + array_start + 1..];
                if let Some(array_end) = Self::find_matching_bracket(events_section, '[', ']') {
                    let events_content = &events_section[..array_end];

                    // Parse each event
                    let mut pos = 0;
                    while pos < events_content.len() {
                        if let Some(obj_start) = events_content[pos..].find('{') {
                            let obj_content = &events_content[pos + obj_start + 1..];
                            if let Some(obj_end) =
                                Self::find_matching_bracket(obj_content, '{', '}')
                            {
                                let event_json = &obj_content[..obj_end];
                                if let Ok(event) = Self::parse_event(event_json) {
                                    abi.events.push(event);
                                }
                                pos = pos + obj_start + obj_end + 2;
                            } else {
                                break;
                            }
                        } else {
                            break;
                        }
                    }
                }
            }
        }

        Ok(abi)
    }

    fn find_matching_bracket(s: &str, open: char, close: char) -> Option<usize> {
        let mut depth = 0;
        let mut in_string = false;
        let mut escape_next = false;

        for (i, ch) in s.chars().enumerate() {
            if escape_next {
                escape_next = false;
                continue;
            }

            if ch == '\\' {
                escape_next = true;
                continue;
            }

            if ch == '"' && !escape_next {
                in_string = !in_string;
            }

            if !in_string {
                if ch == open {
                    depth += 1;
                } else if ch == close {
                    if depth == 0 {
                        return Some(i);
                    }
                    depth -= 1;
                }
            }
        }
        None
    }

    fn parse_function(json: &str) -> Result<FunctionAbi, String> {
        let mut function = FunctionAbi {
            name: String::new(),
            selector: 0,
            inputs: Vec::new(),
            outputs: Vec::new(),
        };

        // Parse name
        if let Some(name) = Self::extract_string_value(json, "name") {
            function.name = name;
        }

        // Parse selector
        if let Some(selector) = Self::extract_number_value(json, "selector") {
            function.selector = selector as u8;
        }

        // Parse inputs
        if let Some(inputs_start) = json.find("\"inputs\"") {
            if let Some(array_start) = json[inputs_start..].find('[') {
                let inputs_section = &json[inputs_start + array_start + 1..];
                if let Some(array_end) = Self::find_matching_bracket(inputs_section, '[', ']') {
                    let inputs_content = &inputs_section[..array_end];
                    function.inputs = Self::parse_function_params(inputs_content);
                }
            }
        }

        // Parse outputs
        if let Some(outputs_start) = json.find("\"outputs\"") {
            if let Some(array_start) = json[outputs_start..].find('[') {
                let outputs_section = &json[outputs_start + array_start + 1..];
                if let Some(array_end) = Self::find_matching_bracket(outputs_section, '[', ']') {
                    let outputs_content = &outputs_section[..array_end];
                    function.outputs = Self::parse_param_types(outputs_content);
                }
            }
        }

        Ok(function)
    }

    fn parse_event(json: &str) -> Result<EventAbi, String> {
        let mut event = EventAbi {
            name: String::new(),
            inputs: Vec::new(),
        };

        // Parse name
        if let Some(name) = Self::extract_string_value(json, "name") {
            event.name = name;
        }

        // Parse inputs
        if let Some(inputs_start) = json.find("\"inputs\"") {
            if let Some(array_start) = json[inputs_start..].find('[') {
                let inputs_section = &json[inputs_start + array_start + 1..];
                if let Some(array_end) = Self::find_matching_bracket(inputs_section, '[', ']') {
                    let inputs_content = &inputs_section[..array_end];
                    event.inputs = Self::parse_event_params(inputs_content);
                }
            }
        }

        Ok(event)
    }

    fn parse_function_params(json: &str) -> Vec<FunctionParam> {
        let mut params = Vec::new();
        let mut pos = 0;

        while pos < json.len() {
            if let Some(obj_start) = json[pos..].find('{') {
                let obj_content = &json[pos + obj_start + 1..];
                if let Some(obj_end) = Self::find_matching_bracket(obj_content, '{', '}') {
                    let param_json = &obj_content[..obj_end];

                    let mut param = FunctionParam {
                        name: String::new(),
                        kind: ParamType::Bytes,
                    };

                    if let Some(name) = Self::extract_string_value(param_json, "name") {
                        param.name = name;
                    }

                    if let Some(type_str) = Self::extract_string_value(param_json, "type") {
                        param.kind = ParamType::from_json_string(&type_str);
                    }

                    params.push(param);
                    pos = pos + obj_start + obj_end + 2;
                } else {
                    break;
                }
            } else {
                break;
            }
        }

        params
    }

    fn parse_event_params(json: &str) -> Vec<EventParam> {
        let mut params = Vec::new();
        let mut pos = 0;

        while pos < json.len() {
            if let Some(obj_start) = json[pos..].find('{') {
                let obj_content = &json[pos + obj_start + 1..];
                if let Some(obj_end) = Self::find_matching_bracket(obj_content, '{', '}') {
                    let param_json = &obj_content[..obj_end];

                    let mut param = EventParam {
                        name: String::new(),
                        kind: ParamType::Bytes,
                        indexed: false,
                    };

                    if let Some(name) = Self::extract_string_value(param_json, "name") {
                        param.name = name;
                    }

                    if let Some(type_str) = Self::extract_string_value(param_json, "type") {
                        param.kind = ParamType::from_json_string(&type_str);
                    }

                    if let Some(indexed) = Self::extract_bool_value(param_json, "indexed") {
                        param.indexed = indexed;
                    }

                    params.push(param);
                    pos = pos + obj_start + obj_end + 2;
                } else {
                    break;
                }
            } else {
                break;
            }
        }

        params
    }

    fn parse_param_types(json: &str) -> Vec<ParamType> {
        let mut types = Vec::new();
        let mut pos = 0;

        while pos < json.len() {
            if let Some(obj_start) = json[pos..].find('{') {
                let obj_content = &json[pos + obj_start + 1..];
                if let Some(obj_end) = Self::find_matching_bracket(obj_content, '{', '}') {
                    let type_json = &obj_content[..obj_end];

                    if let Some(type_str) = Self::extract_string_value(type_json, "type") {
                        types.push(ParamType::from_json_string(&type_str));
                    }

                    pos = pos + obj_start + obj_end + 2;
                } else {
                    break;
                }
            } else {
                break;
            }
        }

        types
    }

    fn extract_string_value(json: &str, key: &str) -> Option<String> {
        let search_key = format!("\"{key}\"");
        if let Some(key_pos) = json.find(&search_key) {
            let after_key = &json[key_pos + search_key.len()..];
            if let Some(colon) = after_key.find(':') {
                let after_colon = &after_key[colon + 1..];
                if let Some(quote1) = after_colon.find('"') {
                    if let Some(quote2) = after_colon[quote1 + 1..].find('"') {
                        return Some(after_colon[quote1 + 1..quote1 + 1 + quote2].to_string());
                    }
                }
            }
        }
        None
    }

    fn extract_number_value(json: &str, key: &str) -> Option<i64> {
        let search_key = format!("\"{key}\"");
        if let Some(key_pos) = json.find(&search_key) {
            let after_key = &json[key_pos + search_key.len()..];
            if let Some(colon) = after_key.find(':') {
                let after_colon = &after_key[colon + 1..].trim();
                let mut num_str = String::new();
                for ch in after_colon.chars() {
                    if ch.is_numeric() || ch == '-' {
                        num_str.push(ch);
                    } else if ch == ',' || ch == '}' || ch == ']' {
                        break;
                    }
                }
                if !num_str.is_empty() {
                    return num_str.parse().ok();
                }
            }
        }
        None
    }

    fn extract_bool_value(json: &str, key: &str) -> Option<bool> {
        let search_key = format!("\"{key}\"");
        if let Some(key_pos) = json.find(&search_key) {
            let after_key = &json[key_pos + search_key.len()..];
            if let Some(colon) = after_key.find(':') {
                let after_colon = &after_key[colon + 1..].trim();
                if after_colon.starts_with("true") {
                    return Some(true);
                } else if after_colon.starts_with("false") {
                    return Some(false);
                }
            }
        }
        None
    }
}

impl Default for ContractAbi {
    fn default() -> Self {
        Self::new()
    }
}
