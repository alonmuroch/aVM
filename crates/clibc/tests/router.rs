use clibc::router::{FuncCall, decode_calls, route};
use types::{Address, Result};

#[test]
fn test_decode_single_call() {
    let input = [0x01, 0x08, 100, 0, 0, 0, 42, 0, 0, 0];
    let mut buf = [FuncCall {
        selector: 0,
        args: &[],
    }; 1];

    let count = decode_calls(&input, &mut buf);
    assert_eq!(count, 1);
    assert_eq!(buf[0].selector, 0x01);
    assert_eq!(buf[0].args, &[100, 0, 0, 0, 42, 0, 0, 0]);
}

#[test]
fn test_decode_multiple_calls() {
    let input = [0x01, 0x02, 1, 2, 0x02, 0x03, 3, 4, 5];
    let mut buf = [FuncCall {
        selector: 0,
        args: &[],
    }; 2];

    let count = decode_calls(&input, &mut buf);
    assert_eq!(count, 2);
    assert_eq!(buf[0].selector, 0x01);
    assert_eq!(buf[0].args, &[1, 2]);
    assert_eq!(buf[1].selector, 0x02);
    assert_eq!(buf[1].args, &[3, 4, 5]);
}

#[test]
fn test_route_with_two_calls() {
    let input = [0x10, 0x01, 42, 0x20, 0x02, 1, 2];

    let to = Address([0u8; 20]);
    let from = Address([0u8; 20]);

    let result = route(&input, to, from, |_to, _from, call| match call.selector {
        0x10 => {
            assert_eq!(call.args, &[42]);
            Result::new(true, 10)
        }
        0x20 => {
            assert_eq!(call.args, &[1, 2]);
            Result::new(false, 20)
        }
        _ => Result::new(false, 999),
    });

    assert_eq!(result, Result::new(false, 20));
}

#[test]
#[should_panic(expected = "vm_panic: decode: incomplete header")]
fn test_decode_incomplete_header_panics() {
    let input = [0x01];
    let mut buf = [FuncCall {
        selector: 0,
        args: &[],
    }; 1];
    decode_calls(&input, &mut buf);
}

#[test]
#[should_panic(expected = "vm_panic: decode: args too short")]
fn test_decode_args_too_short_panics() {
    let input = [0x01, 0x04, 1, 2]; // 4 bytes expected, only 2 given
    let mut buf = [FuncCall {
        selector: 0,
        args: &[],
    }; 1];
    decode_calls(&input, &mut buf);
}
