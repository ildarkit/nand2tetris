use std::collections::HashMap;

pub fn dest(mn: &str) -> String {
    // a d m order: A D M -> bits: d1 d2 d3 but Hack table: ADM -> bits d1 d2 d3 as (A D M)
    // We'll map as in nand2tetris: dest bits are (a,d,m) but order output is d1 d2 d3 where d1 = A, d2 = D, d3 = M?
    // Correct mapping: dest bits are (d1,d2,d3) == (A?1:0, D?1:0, M?1:0) per specification given (we'll follow classic)
    let mut a = '0';
    let mut d = '0';
    let mut m = '0';
    for ch in mn.chars() {
        match ch {
            'A' => a = '1',
            'D' => d = '1',
            'M' => m = '1',
            _ => {}
        }
    }
    format!("{}{}{}", a, d, m)
}

pub fn jump(mn: &str) -> String {
    let mut map = HashMap::new();
    map.insert("", "000");
    map.insert("JGT", "001");
    map.insert("JEQ", "010");
    map.insert("JGE", "011");
    map.insert("JLT", "100");
    map.insert("JNE", "101");
    map.insert("JLE", "110");
    map.insert("JMP", "111");
    map.get(&mn.to_uppercase()[..]).unwrap_or(&"000").to_string()
}

pub fn comp(mn: &str) -> String {
    // canonicalize whitespace and case
    let key = mn.replace(' ', "").replace("+", "+").replace("-", "-").replace("|", "|").replace("&", "&").to_uppercase();
    // mapping table: comp -> 7 bits (a c1..c6)
    let table: &[(&str, &str)] = &[
        ("0","0101010"), ("1","0111111"), ("-1","0111010"),
        ("D","0001100"), ("A","0110000"), ("M","1110000"),
        ("!D","0001101"), ("!A","0110001"), ("!M","1110001"),
        ("-D","0001111"), ("-A","0110011"), ("-M","1110011"),
        ("D+1","0011111"), ("A+1","0110111"), ("M+1","1110111"),
        ("D-1","0001110"), ("A-1","0110010"), ("M-1","1110010"),
        ("D+A","0000010"), ("D+M","1000010"), ("D-A","0010011"),
        ("D-M","1010011"), ("A-D","0000111"), ("M-D","1000111"),
        ("D&A","0000000"), ("D&M","1000000"), ("D|A","0010101"),
        ("D|M","1010101"),
    ];
    for (k,v) in table {
        if key == *k { return v.to_string(); }
    }
    // fallback: return zeros (shouldn't happen for valid mnemonics)
    "0000000".to_string()
}

