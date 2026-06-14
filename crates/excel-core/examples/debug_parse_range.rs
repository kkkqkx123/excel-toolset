use excel_core::utils::cell_ref;

fn main() {
    let result = cell_ref::parse_range("A4:C4");
    println!("parse_range(\"A4:C4\") = {:?}", result);

    let result2 = cell_ref::parse_range("A5:C5");
    println!("parse_range(\"A5:C5\") = {:?}", result2);

    let result3 = cell_ref::parse_range("A1:C1");
    println!("parse_range(\"A1:C1\") = {:?}", result3);
}
