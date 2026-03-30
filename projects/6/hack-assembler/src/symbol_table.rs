use std::collections::HashMap;

pub struct SymbolTable {
    table: HashMap<String, u16>,
}

impl SymbolTable {
    pub fn new() -> Self {
        let mut t = HashMap::new();
        // предопределённые символы (символы из спецификации Hack)
        for i in 0..16 {
            t.insert(format!("R{}", i), i);
        }
        t.insert("SP".to_string(), 0);
        t.insert("LCL".to_string(), 1);
        t.insert("ARG".to_string(), 2);
        t.insert("THIS".to_string(), 3);
        t.insert("THAT".to_string(), 4);
        t.insert("SCREEN".to_string(), 16384);
        t.insert("KBD".to_string(), 24576);

        SymbolTable { table: t }
    }

    pub fn add_entry(&mut self, symbol: &str, address: u16) {
        self.table.insert(symbol.to_string(), address);
    }

    pub fn contains(&self, symbol: &str) -> bool {
        self.table.contains_key(symbol)
    }

    pub fn get_address(&self, symbol: &str) -> Option<u16> {
        self.table.get(symbol).cloned()
    }
}

