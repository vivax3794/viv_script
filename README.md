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

Install build dependencies
```bash
sudo apt install llvm-12 zlib1g-dev libclang-common-12-dev build-essential
```

Create your test file, we will use `test.viv` for this example
```
print "Hello World!";
```

Compile your amazing code!
```
cargo run -- test.viv test
```

Now you can run it! 
```bash
./test
```