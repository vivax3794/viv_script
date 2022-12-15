
#[test]
fn test_string() {
    const CODE: &str = "
    fn main() -> Num {
        print \"Hello World\";

        return 0;
    }
    ";

    let file = temp_file::with_contents(CODE.as_bytes());

    assert_cli::Assert::main_binary()
        .with_args(&["run", file.path().to_str().unwrap()])
        .stdout().contains("Hello World")
        .unwrap();
}

#[test]
fn test_number() {
    const CODE: &str = "
    fn main() -> Num {
        print 1;

        return 0;
    }
    ";

    let file = temp_file::with_contents(CODE.as_bytes());

    assert_cli::Assert::main_binary()
        .with_args(&["run", file.path().to_str().unwrap()])
        .stdout().contains("1")
        .unwrap();
}

#[test]
fn test_bool_true() {
    const CODE: &str = "
    fn main() -> Num {
        print true;

        return 0;
    }
    ";

    let file = temp_file::with_contents(CODE.as_bytes());

    assert_cli::Assert::main_binary()
        .with_args(&["run", file.path().to_str().unwrap()])
        .stdout().contains("true")
        .unwrap();
}

#[test]
fn test_bool_false() {
    const CODE: &str = "
    fn main() -> Num {
        print false;

        return 0;
    }
    ";

    let file = temp_file::with_contents(CODE.as_bytes());

    assert_cli::Assert::main_binary()
        .with_args(&["run", file.path().to_str().unwrap()])
        .stdout().contains("false")
        .unwrap();
}