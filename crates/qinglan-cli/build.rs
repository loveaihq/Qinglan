// An 8 MB main-thread stack (L1 task 17): a stack overflow cannot be caught, so the page code gets room well above
// what the recursion limits in the engine need (the default is 1 MB).
fn main() {
    println!("cargo:rustc-link-arg-bins=/STACK:8388608");
}
