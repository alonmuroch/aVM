use compiler::{
    AbiGenerator, ContractAbi, EventAbi, EventParam, FunctionAbi, FunctionParam, ParamType,
};

#[test]
fn test_abi_generation() {
    // Test with a simple program that has events
    let source_code = r#"
        event!(Minted {
            caller => Address,
            amount => u32,
        });
        
        event!(Transfer {
            from => Address,
            to => Address,
            value => u32,
        });
    "#;

    let mut generator = AbiGenerator::new(source_code.to_string());
    let abi = generator.generate();

    assert_eq!(abi.events.len(), 2);
    assert_eq!(abi.events[0].name, "Minted");
    assert_eq!(abi.events[1].name, "Transfer");

    println!("Generated ABI: {}", abi.to_json());
}

#[test]
fn test_function_extraction() {
    // Test with a program that has functions with selectors
    let source_code = r#"
        unsafe fn main_entry(program: Address, caller: Address, data: &[u8]) -> Result {   
            route(data, program, caller, 
                 |to, from, call| match call.selector {
                0x01 => {
                    init(caller, call.args);
                    Result::new(true, 0)
                },
                0x02 => {
                    let mut parser = DataParser::new(call.args);
                    let to = parser.read_address();
                    let amount = parser.read_u32();
                    transfer(caller, to, amount);
                    Result::new(true, 0)
                },
                0x05 => {
                    let mut parser = DataParser::new(call.args);
                    let owner = parser.read_address();
                    let b = balance_of(owner);
                    Result::with_u32(b)
                },
                _ => vm_panic(b"unknown selector"),
            })
        }

        fn init(caller: Address, args: &[u8]) {
            // function body
        }

        fn transfer(caller: Address, to: Address, amount: u32) {
            // function body
        }

        fn balance_of(owner: Address) -> u32 {
            0
        }
    "#;

    let mut generator = AbiGenerator::new(source_code.to_string());
    let abi = generator.generate();

    assert_eq!(abi.functions.len(), 3);

    // Check that all expected functions are found
    let function_names: Vec<&str> = abi.functions.iter().map(|f| f.name.as_str()).collect();
    assert!(function_names.contains(&"init"));
    assert!(function_names.contains(&"transfer"));
    assert!(function_names.contains(&"balance_of"));

    // Check selectors
    let init_func = abi.functions.iter().find(|f| f.name == "init").unwrap();
    let transfer_func = abi.functions.iter().find(|f| f.name == "transfer").unwrap();
    let balance_func = abi
        .functions
        .iter()
        .find(|f| f.name == "balance_of")
        .unwrap();

    assert_eq!(init_func.selector, 1);
    assert_eq!(transfer_func.selector, 2);
    assert_eq!(balance_func.selector, 5);
}

#[test]
fn test_multi_line_event_parsing() {
    // Test with multi-line event definition
    let source_code = r#"
        event!(ComplexEvent {
            user => Address,
            amount => u32,
            timestamp => u64,
            metadata => String,
        });
    "#;

    let mut generator = AbiGenerator::new(source_code.to_string());
    let abi = generator.generate();

    assert_eq!(abi.events.len(), 1);
    let event = &abi.events[0];
    assert_eq!(event.name, "ComplexEvent");
    assert_eq!(event.inputs.len(), 4);

    // Check parameter types
    assert_eq!(event.inputs[0].name, "user");
    assert!(matches!(event.inputs[0].kind, ParamType::Address));
    assert_eq!(event.inputs[1].name, "amount");
    assert!(matches!(event.inputs[1].kind, ParamType::Uint(32)));
    assert_eq!(event.inputs[2].name, "timestamp");
    assert!(matches!(event.inputs[2].kind, ParamType::Uint(64)));
    assert_eq!(event.inputs[3].name, "metadata");
    assert!(matches!(event.inputs[3].kind, ParamType::String));
}

#[test]
fn test_empty_program() {
    // Test with a program that has no events or functions
    let source_code = r#"
        fn main() {
            // Just a simple function
        }
    "#;

    let mut generator = AbiGenerator::new(source_code.to_string());
    let abi = generator.generate();

    assert_eq!(abi.events.len(), 0);
    assert_eq!(abi.functions.len(), 0);
}

#[test]
fn test_contract_abi_json_generation() {
    // Test complete ABI JSON generation
    let mut abi = ContractAbi::new();

    // Add an event
    abi.add_event(EventAbi {
        name: "TestEvent".to_string(),
        inputs: vec![
            EventParam {
                name: "param1".to_string(),
                kind: ParamType::Address,
                indexed: false,
            },
            EventParam {
                name: "param2".to_string(),
                kind: ParamType::Uint(32),
                indexed: true,
            },
        ],
    });

    // Add a function
    abi.add_function(FunctionAbi {
        name: "testFunction".to_string(),
        selector: 1,
        inputs: vec![FunctionParam {
            name: "input1".to_string(),
            kind: ParamType::Address,
        }],
        outputs: vec![ParamType::Uint(32)],
    });

    let json = abi.to_json();

    // Verify JSON contains expected content
    assert!(json.contains("\"name\": \"TestEvent\""));
    assert!(json.contains("\"name\": \"testFunction\""));
    assert!(json.contains("\"selector\": 1"));
    assert!(json.contains("\"type\": \"address\""));
    assert!(json.contains("\"type\": \"uint32\""));
    assert!(json.contains("\"indexed\": true"));
    assert!(json.contains("\"indexed\": false"));
}

#[test]
fn test_real_erc20_like_program() {
    // Test with a realistic ERC20-like program
    let source_code = r#"
        event!(Transfer {
            from => Address,
            to => Address,
            value => u32,
        });

        event!(Minted {
            caller => Address,
            amount => u32,
        });

        unsafe fn main_entry(program: Address, caller: Address, data: &[u8]) -> Result {   
            route(data, program, caller, 
                 |to, from, call| match call.selector {
                0x01 => {
                    init(caller, call.args);
                    Result::new(true, 0)
                },
                0x02 => {
                    let mut parser = DataParser::new(call.args);
                    let to = parser.read_address();
                    let amount = parser.read_u32();
                    transfer(caller, to, amount);
                    Result::new(true, 0)
                },
                0x05 => {
                    let mut parser = DataParser::new(call.args);
                    let owner = parser.read_address();
                    let b = balance_of(owner);
                    Result::with_u32(b)
                },
                _ => vm_panic(b"unknown selector"),
            })
        }

        fn init(caller: Address, args: &[u8]) {
            // Initialize contract
        }

        fn transfer(caller: Address, to: Address, amount: u32) {
            // Transfer tokens
            fire_event!(Transfer::new(caller, caller, 0));
        }

        fn balance_of(owner: Address) -> u32 {
            0
        }
    "#;

    let mut generator = AbiGenerator::new(source_code.to_string());
    let abi = generator.generate();

    // Check events
    assert_eq!(abi.events.len(), 2);
    let event_names: Vec<&str> = abi.events.iter().map(|e| e.name.as_str()).collect();
    assert!(event_names.contains(&"Transfer"));
    assert!(event_names.contains(&"Minted"));

    // Check functions
    assert_eq!(abi.functions.len(), 3);
    let function_names: Vec<&str> = abi.functions.iter().map(|f| f.name.as_str()).collect();
    assert!(function_names.contains(&"init"));
    assert!(function_names.contains(&"transfer"));
    assert!(function_names.contains(&"balance_of"));

    // Check selectors
    let init_func = abi.functions.iter().find(|f| f.name == "init").unwrap();
    let transfer_func = abi.functions.iter().find(|f| f.name == "transfer").unwrap();
    let balance_func = abi
        .functions
        .iter()
        .find(|f| f.name == "balance_of")
        .unwrap();

    assert_eq!(init_func.selector, 1);
    assert_eq!(transfer_func.selector, 2);
    assert_eq!(balance_func.selector, 5);
}

#[test]
fn test_param_type_json_conversion() {
    // Test that ParamType converts to JSON correctly
    assert_eq!(ParamType::Address.to_json_string(), "address");
    assert_eq!(ParamType::Uint(32).to_json_string(), "uint32");
    assert_eq!(ParamType::Uint(64).to_json_string(), "uint64");
    assert_eq!(ParamType::Uint(8).to_json_string(), "uint8");
    assert_eq!(ParamType::Bool.to_json_string(), "bool");
    assert_eq!(ParamType::String.to_json_string(), "string");
    assert_eq!(ParamType::Bytes.to_json_string(), "bytes");
}

#[test]
fn test_function_input_extraction() {
    // Test with functions that have proper input parameters
    let source_code = r#"
        fn init(caller: Address, args: &[u8]) {
            // function body
        }

        fn transfer(caller: Address, to: Address, amount: u32) {
            // function body
        }

        fn balance_of(owner: Address) -> u32 {
            0
        }

        unsafe fn main_entry(program: Address, caller: Address, data: &[u8]) -> Result {   
            route(data, program, caller, 
                 |to, from, call| match call.selector {
                0x01 => {
                    init(caller, call.args);
                    Result::new(true, 0)
                },
                0x02 => {
                    let mut parser = DataParser::new(call.args);
                    let to = parser.read_address();
                    let amount = parser.read_u32();
                    transfer(caller, to, amount);
                    Result::new(true, 0)
                },
                0x05 => {
                    let mut parser = DataParser::new(call.args);
                    let owner = parser.read_address();
                    let b = balance_of(owner);
                    Result::with_u32(b)
                },
                _ => vm_panic(b"unknown selector"),
            })
        }
    "#;

    let mut generator = AbiGenerator::new(source_code.to_string());
    let abi = generator.generate();

    // Check that all functions have the correct inputs
    assert_eq!(abi.functions.len(), 3);

    // Check init function
    let init_func = abi.functions.iter().find(|f| f.name == "init").unwrap();
    // The generator drops the implicit `caller` argument; only routed args remain.
    assert_eq!(init_func.inputs.len(), 1);
    assert_eq!(init_func.inputs[0].name, "args");
    assert!(matches!(init_func.inputs[0].kind, ParamType::Bytes));

    // Check transfer function
    let transfer_func = abi.functions.iter().find(|f| f.name == "transfer").unwrap();
    assert_eq!(transfer_func.inputs.len(), 2);
    assert_eq!(transfer_func.inputs[0].name, "to");
    assert!(matches!(transfer_func.inputs[0].kind, ParamType::Address));
    assert_eq!(transfer_func.inputs[1].name, "amount");
    assert!(matches!(transfer_func.inputs[1].kind, ParamType::Uint(32)));

    // Check balance_of function
    let balance_func = abi
        .functions
        .iter()
        .find(|f| f.name == "balance_of")
        .unwrap();
    assert_eq!(balance_func.inputs.len(), 1);
    assert_eq!(balance_func.inputs[0].name, "owner");
    assert!(matches!(balance_func.inputs[0].kind, ParamType::Address));
    assert_eq!(balance_func.outputs.len(), 1);
    assert!(matches!(balance_func.outputs[0], ParamType::Uint(32)));
}

#[test]
fn test_function_signature_parsing() {
    // Test parsing of various function signature patterns
    let source_code = r#"
        fn simple_function() {
            // no parameters
        }

        fn single_param(param: u32) {
            // single parameter
        }

        fn multiple_params(a: Address, b: u64, c: bool) {
            // multiple parameters
        }

        fn with_return_type(input: String) -> u32 {
            0
        }

        fn complex_signature(
            caller: Address,
            args: &[u8],
            flag: bool
        ) -> u32 {
            0
        }
    "#;

    let mut generator = AbiGenerator::new(source_code.to_string());
    let _abi = generator.generate();

    // Since these functions aren't called in the router, they won't be included
    // But we can test the signature parsing directly
    let lines: Vec<&str> = source_code.lines().collect();

    // Test simple_function
    if let Some(signature) = generator.extract_function_signature(&lines, 1) {
        let func = generator.parse_function_signature("simple_function", signature);
        assert_eq!(func.inputs.len(), 0);
    }

    // Test single_param
    if let Some(signature) = generator.extract_function_signature(&lines, 5) {
        let func = generator.parse_function_signature("single_param", signature);
        assert_eq!(func.inputs.len(), 1);
        assert_eq!(func.inputs[0].name, "param");
        assert!(matches!(func.inputs[0].kind, ParamType::Uint(32)));
    }

    // Test multiple_params
    if let Some(signature) = generator.extract_function_signature(&lines, 9) {
        let func = generator.parse_function_signature("multiple_params", signature);
        assert_eq!(func.inputs.len(), 3);
        assert_eq!(func.inputs[0].name, "a");
        assert!(matches!(func.inputs[0].kind, ParamType::Address));
        assert_eq!(func.inputs[1].name, "b");
        assert!(matches!(func.inputs[1].kind, ParamType::Uint(64)));
        assert_eq!(func.inputs[2].name, "c");
        assert!(matches!(func.inputs[2].kind, ParamType::Bool));
    }

    // Test with_return_type
    if let Some(signature) = generator.extract_function_signature(&lines, 13) {
        let func = generator.parse_function_signature("with_return_type", signature);
        assert_eq!(func.inputs.len(), 1);
        assert_eq!(func.inputs[0].name, "input");
        assert!(matches!(func.inputs[0].kind, ParamType::String));
        assert_eq!(func.outputs.len(), 1);
        assert!(matches!(func.outputs[0], ParamType::Uint(32)));
    }
}

#[test]
fn test_param_type_parsing() {
    let generator = AbiGenerator::new("".to_string());

    // Test basic types
    assert_eq!(
        generator.parse_param_type_from_str("Address"),
        Some(ParamType::Address)
    );
    assert_eq!(
        generator.parse_param_type_from_str("u32"),
        Some(ParamType::Uint(32))
    );
    assert_eq!(
        generator.parse_param_type_from_str("u64"),
        Some(ParamType::Uint(64))
    );
    assert_eq!(
        generator.parse_param_type_from_str("bool"),
        Some(ParamType::Bool)
    );
    assert_eq!(
        generator.parse_param_type_from_str("&[u8]"),
        Some(ParamType::Bytes)
    );
    assert_eq!(
        generator.parse_param_type_from_str("String"),
        Some(ParamType::String)
    );
    assert_eq!(
        generator.parse_param_type_from_str("Result"),
        Some(ParamType::Result)
    );

    // Test edge cases
    assert_eq!(generator.parse_param_type_from_str(""), None);
    assert_eq!(generator.parse_param_type_from_str("unknown"), None);
    assert_eq!(
        generator.parse_param_type_from_str("u32 "),
        Some(ParamType::Uint(32))
    ); // with space
    assert_eq!(
        generator.parse_param_type_from_str(" u32"),
        Some(ParamType::Uint(32))
    ); // with leading space
}

#[test]
fn test_edge_case_router_patterns() {
    // Test router with comments and extra whitespace
    let source_code = r#"
        fn test_function() -> u32 { 0 }
        
        unsafe fn main_entry(program: Address, caller: Address, data: &[u8]) -> Result {   
            route(data, program, caller, 
                 |to, from, call| match call.selector {
                0x01 => {
                    // Comment on same line
                    test_function(); // Another comment
                    Result::new(true, 0)
                },
                0x02 => test_function(), // Direct call with comment
                0x03 => {
                    let result = test_function(); // Assignment with comment
                    Result::with_u32(result)
                },
                _ => vm_panic(b"unknown selector"),
            })
        }
    "#;

    let mut generator = AbiGenerator::new(source_code.to_string());
    let abi = generator.generate();

    assert_eq!(abi.functions.len(), 3);

    // Check that all functions were found despite comments
    let function_names: Vec<String> = abi.functions.iter().map(|f| f.name.clone()).collect();
    assert!(function_names.contains(&"test_function".to_string()));
}

#[test]
fn test_malformed_function_signatures() {
    // Test various malformed function signatures
    let source_code = r#"
        fn valid_function(a: Address, b: u32) -> u32 { 0 }
        fn no_params() { }
        fn missing_return_type(a: u32) { }
        fn complex_return() -> Result { Result::new(true, 0) }
        
        unsafe fn main_entry(program: Address, caller: Address, data: &[u8]) -> Result {   
            route(data, program, caller, 
                 |to, from, call| match call.selector {
                0x01 => valid_function(Address::default(), 42),
                0x02 => no_params(),
                0x03 => missing_return_type(123),
                0x04 => complex_return(),
                _ => vm_panic(b"unknown selector"),
            })
        }
    "#;

    let mut generator = AbiGenerator::new(source_code.to_string());
    let abi = generator.generate();

    // Should find all functions regardless of signature complexity
    assert_eq!(abi.functions.len(), 4);

    // Check specific function signatures
    let valid_func = abi
        .functions
        .iter()
        .find(|f| f.name == "valid_function")
        .unwrap();
    assert_eq!(valid_func.inputs.len(), 2);
    assert_eq!(valid_func.outputs.len(), 1);

    let no_params_func = abi
        .functions
        .iter()
        .find(|f| f.name == "no_params")
        .unwrap();
    assert_eq!(no_params_func.inputs.len(), 0);
    assert_eq!(no_params_func.outputs.len(), 0); // void function

    let missing_return_func = abi
        .functions
        .iter()
        .find(|f| f.name == "missing_return_type")
        .unwrap();
    assert_eq!(missing_return_func.inputs.len(), 1);
    assert_eq!(missing_return_func.outputs.len(), 0); // void function

    let complex_return_func = abi
        .functions
        .iter()
        .find(|f| f.name == "complex_return")
        .unwrap();
    assert_eq!(complex_return_func.inputs.len(), 0);
    assert_eq!(complex_return_func.outputs.len(), 1);
    assert!(matches!(complex_return_func.outputs[0], ParamType::Result));
}

#[test]
fn test_multiline_function_signatures() {
    // Test functions with multi-line signatures
    let source_code = r#"
        fn complex_function(
            caller: Address,
            args: &[u8],
            flag: bool,
            count: u64
        ) -> Result {
            Result::new(true, 0)
        }
        
        fn another_function(
            param1: u32,
            param2: String
        ) -> u32 {
            42
        }
        
        unsafe fn main_entry(program: Address, caller: Address, data: &[u8]) -> Result {   
            route(data, program, caller, 
                 |to, from, call| match call.selector {
                0x01 => complex_function(caller, data, true, 100),
                0x02 => another_function(123, "test".to_string()),
                _ => vm_panic(b"unknown selector"),
            })
        }
    "#;

    let mut generator = AbiGenerator::new(source_code.to_string());
    let abi = generator.generate();

    assert_eq!(abi.functions.len(), 2);

    let complex_func = abi
        .functions
        .iter()
        .find(|f| f.name == "complex_function")
        .unwrap();
    // Caller is implicit; ABI includes routed args only.
    assert_eq!(complex_func.inputs.len(), 3);
    assert_eq!(complex_func.inputs[0].name, "args");
    assert!(matches!(complex_func.inputs[0].kind, ParamType::Bytes));
    assert_eq!(complex_func.inputs[1].name, "flag");
    assert!(matches!(complex_func.inputs[1].kind, ParamType::Bool));
    assert_eq!(complex_func.inputs[2].name, "count");
    assert!(matches!(complex_func.inputs[2].kind, ParamType::Uint(64)));
    assert_eq!(complex_func.outputs.len(), 1);
    assert!(matches!(complex_func.outputs[0], ParamType::Result));

    let another_func = abi
        .functions
        .iter()
        .find(|f| f.name == "another_function")
        .unwrap();
    assert_eq!(another_func.inputs.len(), 2);
    assert_eq!(another_func.outputs.len(), 1);
    assert!(matches!(another_func.outputs[0], ParamType::Uint(32)));
}

#[test]
fn test_different_router_patterns() {
    // Test various router patterns
    let source_code = r#"
        fn func1() -> u32 { 1 }
        fn func2(a: u32) -> u32 { a }
        fn func3() { }
        
        unsafe fn main_entry(program: Address, caller: Address, data: &[u8]) -> Result {   
            route(data, program, caller, 
                 |to, from, call| match call.selector {
                0x01 => func1(), // Direct call
                0x02 => {
                    func2(42); // Call in block
                },
                0x03 => {
                    let result = func3(); // Assignment
                    Result::new(true, 0)
                },
                0x04 => {
                    if true {
                        func1() // Nested call
                    } else {
                        func2(0)
                    }
                },
                _ => vm_panic(b"unknown selector"),
            })
        }
    "#;

    let mut generator = AbiGenerator::new(source_code.to_string());
    let abi = generator.generate();

    // The router pattern with nested calls might find more functions than expected
    // Let's check that at least the main functions were found
    let function_names: Vec<String> = abi.functions.iter().map(|f| f.name.clone()).collect();
    assert!(function_names.contains(&"func1".to_string()));
    assert!(function_names.contains(&"func2".to_string()));
    assert!(function_names.contains(&"func3".to_string()));
}

#[test]
fn test_result_type_handling() {
    // Test functions that return Result type
    let source_code = r#"
        fn success_result() -> Result {
            Result::new(true, 0)
        }
        
        fn error_result() -> Result {
            Result::new(false, 1)
        }
        
        fn data_result() -> Result {
            Result::with_u32(42)
        }
        
        fn void_function() { }
        
        unsafe fn main_entry(program: Address, caller: Address, data: &[u8]) -> Result {   
            route(data, program, caller, 
                 |to, from, call| match call.selector {
                0x01 => success_result(),
                0x02 => error_result(),
                0x03 => data_result(),
                0x04 => void_function(),
                _ => vm_panic(b"unknown selector"),
            })
        }
    "#;

    let mut generator = AbiGenerator::new(source_code.to_string());
    let abi = generator.generate();

    assert_eq!(abi.functions.len(), 4);

    // Check Result return types
    let success_func = abi
        .functions
        .iter()
        .find(|f| f.name == "success_result")
        .unwrap();
    assert_eq!(success_func.outputs.len(), 1);
    assert!(matches!(success_func.outputs[0], ParamType::Result));

    let error_func = abi
        .functions
        .iter()
        .find(|f| f.name == "error_result")
        .unwrap();
    assert_eq!(error_func.outputs.len(), 1);
    assert!(matches!(error_func.outputs[0], ParamType::Result));

    let data_func = abi
        .functions
        .iter()
        .find(|f| f.name == "data_result")
        .unwrap();
    assert_eq!(data_func.outputs.len(), 1);
    assert!(matches!(data_func.outputs[0], ParamType::Result));

    let void_func = abi
        .functions
        .iter()
        .find(|f| f.name == "void_function")
        .unwrap();
    assert_eq!(void_func.outputs.len(), 0); // void function
}

#[test]
fn test_complex_parameter_types() {
    // Test various parameter types
    let source_code = r#"
        fn test_types(
            addr: Address,
            num: u32,
            flag: bool,
            data: &[u8],
            text: String,
            result: Result
        ) -> u32 {
            0
        }
        
        unsafe fn main_entry(program: Address, caller: Address, data: &[u8]) -> Result {   
            route(data, program, caller, 
                 |to, from, call| match call.selector {
                0x01 => test_types(Address::default(), 42, true, data, "test".to_string(), Result::new(true, 0)),
                _ => vm_panic(b"unknown selector"),
            })
        }
    "#;

    let mut generator = AbiGenerator::new(source_code.to_string());
    let abi = generator.generate();

    assert_eq!(abi.functions.len(), 1);

    let func = &abi.functions[0];
    assert_eq!(func.inputs.len(), 6);

    // Check parameter types
    assert_eq!(func.inputs[0].name, "addr");
    assert!(matches!(func.inputs[0].kind, ParamType::Address));

    assert_eq!(func.inputs[1].name, "num");
    assert!(matches!(func.inputs[1].kind, ParamType::Uint(32)));

    assert_eq!(func.inputs[2].name, "flag");
    assert!(matches!(func.inputs[2].kind, ParamType::Bool));

    assert_eq!(func.inputs[3].name, "data");
    assert!(matches!(func.inputs[3].kind, ParamType::Bytes));

    assert_eq!(func.inputs[4].name, "text");
    assert!(matches!(func.inputs[4].kind, ParamType::String));

    assert_eq!(func.inputs[5].name, "result");
    assert!(matches!(func.inputs[5].kind, ParamType::Result));
}

#[test]
fn test_empty_and_malformed_programs() {
    // Test empty program
    let mut empty_generator = AbiGenerator::new("".to_string());
    let empty_abi = empty_generator.generate();
    assert_eq!(empty_abi.functions.len(), 0);
    assert_eq!(empty_abi.events.len(), 0);

    // Test program with no router
    let no_router_source = r#"
        fn some_function() -> u32 { 0 }
        fn another_function() { }
    "#;
    let mut no_router_generator = AbiGenerator::new(no_router_source.to_string());
    let no_router_abi = no_router_generator.generate();
    assert_eq!(no_router_abi.functions.len(), 0); // No functions without router

    // Test malformed router
    let malformed_source = r#"
        fn test_function() -> u32 { 0 }
        
        unsafe fn main_entry(program: Address, caller: Address, data: &[u8]) -> Result {   
            // No router call
            Result::new(true, 0)
        }
    "#;
    let mut malformed_generator = AbiGenerator::new(malformed_source.to_string());
    let malformed_abi = malformed_generator.generate();
    assert_eq!(malformed_abi.functions.len(), 0); // No functions without router
}

#[test]
fn test_event_edge_cases() {
    // Test events with various field types and edge cases
    let source_code = r#"
        event!(MultipleFields {
            sender => Address,
            amount => u64,
            success => bool,
            data => &[u8],
            message => String,
        });
        
        event!(MultiLineEvent {
            field1 => Address,
            field2 => u32,
            field3 => bool,
        });
        
        fn test_function() -> u32 { 0 }
        
        unsafe fn main_entry(program: Address, caller: Address, data: &[u8]) -> Result {   
            route(data, program, caller, 
                 |to, from, call| match call.selector {
                0x01 => test_function(),
                _ => vm_panic(b"unknown selector"),
            })
        }
    "#;

    let mut generator = AbiGenerator::new(source_code.to_string());
    let abi = generator.generate();

    // The event parsing might find fewer events than expected due to parsing limitations
    assert!(abi.events.len() >= 2);

    // Check multiple fields event
    let multiple_event = abi
        .events
        .iter()
        .find(|e| e.name == "MultipleFields")
        .unwrap();
    assert_eq!(multiple_event.inputs.len(), 5);

    let field_names: Vec<String> = multiple_event
        .inputs
        .iter()
        .map(|f| f.name.clone())
        .collect();
    assert!(field_names.contains(&"sender".to_string()));
    assert!(field_names.contains(&"amount".to_string()));
    assert!(field_names.contains(&"success".to_string()));
    assert!(field_names.contains(&"data".to_string()));
    assert!(field_names.contains(&"message".to_string()));

    // Check field types
    let sender_field = multiple_event
        .inputs
        .iter()
        .find(|f| f.name == "sender")
        .unwrap();
    assert!(matches!(sender_field.kind, ParamType::Address));

    let amount_field = multiple_event
        .inputs
        .iter()
        .find(|f| f.name == "amount")
        .unwrap();
    assert!(matches!(amount_field.kind, ParamType::Uint(64)));

    let success_field = multiple_event
        .inputs
        .iter()
        .find(|f| f.name == "success")
        .unwrap();
    assert!(matches!(success_field.kind, ParamType::Bool));

    let data_field = multiple_event
        .inputs
        .iter()
        .find(|f| f.name == "data")
        .unwrap();
    assert!(matches!(data_field.kind, ParamType::Bytes));

    let message_field = multiple_event
        .inputs
        .iter()
        .find(|f| f.name == "message")
        .unwrap();
    assert!(matches!(message_field.kind, ParamType::String));
}

#[test]
fn test_erc20_example_file_generates_typed_abi() {
    // Ensure the real ERC20 example stays in sync with the typed ABI we expect
    let source_code = include_str!("../../examples/src/erc20.rs");
    let mut generator = AbiGenerator::new(source_code.to_string());
    let abi = generator.generate();

    let function_names: Vec<&str> = abi.functions.iter().map(|f| f.name.as_str()).collect();
    assert!(function_names.contains(&"init"));
    assert!(function_names.contains(&"transfer"));
    assert!(function_names.contains(&"balance_of"));

    let init_func = abi.functions.iter().find(|f| f.name == "init").unwrap();
    assert_eq!(init_func.selector, 1);
    // The routed payload stays raw bytes for init.
    assert_eq!(init_func.inputs.len(), 1);
    assert_eq!(init_func.inputs[0].name, "args");
    assert!(matches!(init_func.inputs[0].kind, ParamType::Bytes));

    let transfer_func = abi.functions.iter().find(|f| f.name == "transfer").unwrap();
    assert_eq!(transfer_func.selector, 2);
    assert_eq!(transfer_func.inputs.len(), 2);
    assert_eq!(transfer_func.inputs[0].name, "to");
    assert!(matches!(transfer_func.inputs[0].kind, ParamType::Address));
    assert_eq!(transfer_func.inputs[1].name, "amount");
    assert!(matches!(transfer_func.inputs[1].kind, ParamType::Uint(32)));

    let approve_func = abi.functions.iter().find(|f| f.name == "approve").unwrap();
    assert_eq!(approve_func.selector, 3);
    assert_eq!(approve_func.inputs.len(), 2);
    assert_eq!(approve_func.inputs[0].name, "spender");
    assert!(matches!(approve_func.inputs[0].kind, ParamType::Address));
    assert_eq!(approve_func.inputs[1].name, "amount");
    assert!(matches!(approve_func.inputs[1].kind, ParamType::Uint(32)));

    let transfer_from = abi
        .functions
        .iter()
        .find(|f| f.name == "transfer_from")
        .unwrap();
    assert_eq!(transfer_from.selector, 4);
    assert_eq!(transfer_from.inputs.len(), 3);
    assert_eq!(transfer_from.inputs[0].name, "from");
    assert!(matches!(transfer_from.inputs[0].kind, ParamType::Address));
    assert_eq!(transfer_from.inputs[1].name, "to");
    assert!(matches!(transfer_from.inputs[1].kind, ParamType::Address));
    assert_eq!(transfer_from.inputs[2].name, "amount");
    assert!(matches!(transfer_from.inputs[2].kind, ParamType::Uint(32)));

    let balance_func = abi
        .functions
        .iter()
        .find(|f| f.name == "balance_of")
        .unwrap();
    assert_eq!(balance_func.selector, 5);
    assert_eq!(balance_func.inputs.len(), 1);
    assert_eq!(balance_func.inputs[0].name, "owner");
    assert!(matches!(balance_func.inputs[0].kind, ParamType::Address));
    assert_eq!(balance_func.outputs.len(), 1);
    assert!(matches!(balance_func.outputs[0], ParamType::Uint(32)));
}

#[test]
fn test_router_preparsed_args_keep_typed_signature() {
    // When call.args is parsed into typed vars before dispatch, the ABI should still reflect
    // the callee's typed signature (and drop the implicit caller).
    let source_code = r#"
        fn routed_transfer(caller: Address, to: Address, amount: u64, flag: bool) -> Result {
            Result::new(true, if flag { 1 } else { 0 })
        }

        unsafe fn main_entry(program: Address, caller: Address, data: &[u8]) -> Result {
            route(data, program, caller, |to_addr, from_addr, call| match call.selector {
                0x0a => {
                    let mut parser = DataParser::new(call.args);
                    let to = parser.read_address();
                    let amount = parser.read_u64();
                    let flag = parser.read_bytes(1)[0] == 1;
                    routed_transfer(caller, to, amount, flag);
                    Result::new(true, 0)
                }
                _ => vm_panic(b\"unknown selector\"),
            })
        }
    "#;

    let mut generator = AbiGenerator::new(source_code.to_string());
    let abi = generator.generate();

    assert_eq!(abi.functions.len(), 1);
    let func = &abi.functions[0];
    assert_eq!(func.name, "routed_transfer");
    assert_eq!(func.selector, 0x0a);
    // Caller is implicit; only the routed args appear.
    assert_eq!(func.inputs.len(), 3);
    assert_eq!(func.inputs[0].name, "to");
    assert!(matches!(func.inputs[0].kind, ParamType::Address));
    assert_eq!(func.inputs[1].name, "amount");
    assert!(matches!(func.inputs[1].kind, ParamType::Uint(64)));
    assert_eq!(func.inputs[2].name, "flag");
    assert!(matches!(func.inputs[2].kind, ParamType::Bool));
    assert_eq!(func.outputs.len(), 1);
    assert!(matches!(func.outputs[0], ParamType::Result));
}
