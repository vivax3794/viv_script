# viv_script

## How To Run

### Ubunto

These instructions are tested on a fresh WSL2 Ubunto install;
You need to install [rust](https://www.rust-lang.org/learn/get-started)

download repo
```bash
git clone https://github.com/vivax3794/viv_script.git
cd viv_script
```

Install build and runtime dependencies
```bash
sudo apt install llvm-14 zlib1g-dev libclang-common-14-dev build-essential
```

Create your test file, we will use `test.viv` for this example
```
fn main() -> Num {
    print "Hello world!";

    return 0;
}
```

Compile your amazing code!
```
cargo run -- run test.viv
```