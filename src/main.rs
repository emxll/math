mod number;

use number::Integer;

fn main() {
    let a: Integer = 0xffff_ffff_ffff_ffff.into();
    let b: Integer = 12.into();
    let c = a + b;
    println!("{c:?}");
    let d: Integer = 0xffff_ffff_ffff_fffe.into();
    let e = c - d;
    println!("{e:?}");
    let f: Integer = 0.into();
    let g = f - e;
    println!("{g:?}");
}
