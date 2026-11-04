fn steps(mut m: i64) -> i64 {
    let mut c = 0i64;
    while m != 1 {
        if m % 2 == 0 { m /= 2; } else { m = 3 * m + 1; }
        c += 1;
    }
    c
}
fn main() {
    let mut total = 0i64;
    for i in 1..1_000_000 { total += steps(i); }
    println!("collatz total: {}", total);
}
