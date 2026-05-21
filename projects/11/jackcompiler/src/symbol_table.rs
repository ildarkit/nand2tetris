use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Static,
    Field,
    Arg,
    Var,
}

#[derive(Debug, Clone)]
struct Symbol {
    type_of: String,
    kind: Kind,
    index: usize,
}

pub struct SymbolTable {
    class_symbols: HashMap<String, Symbol>,
    subroutine_symbols: HashMap<String, Symbol>,
    indices: HashMap<Kind, usize>,
}

impl SymbolTable {
    pub fn new() -> Self {
        let mut indices = HashMap::new();
        indices.insert(Kind::Static, 0);
        indices.insert(Kind::Field, 0);
        indices.insert(Kind::Arg, 0);
        indices.insert(Kind::Var, 0);

        SymbolTable {
            class_symbols: HashMap::new(),
            subroutine_symbols: HashMap::new(),
            indices,
        }
    }

    pub fn reset(&mut self) {
        self.subroutine_symbols.clear();
        self.indices.insert(Kind::Arg, 0);
        self.indices.insert(Kind::Var, 0);
    }

    pub fn define(&mut self, name: &str, type_of: &str, kind: Kind) {
        let index = *self.indices.get(&kind).unwrap_or(&0);
        
        let symbol = Symbol {
            type_of: type_of.to_string(),
            kind,
            index,
        };

        match kind {
            Kind::Static | Kind::Field => {
                self.class_symbols.insert(name.to_string(), symbol);
            }
            Kind::Arg | Kind::Var => {
                self.subroutine_symbols.insert(name.to_string(), symbol);
            }
        }

        self.indices.insert(kind, index + 1);
    }

    pub fn var_count(&self, kind: Kind) -> usize {
        *self.indices.get(&kind).unwrap_or(&0)
    }

    fn look_up(&self, name: &str) -> Option<&Symbol> {
        self.subroutine_symbols.get(name).or_else(|| self.class_symbols.get(name))
    }

    pub fn kind_of(&self, name: &str) -> Option<Kind> {
        self.look_up(name).map(|s| s.kind)
    }

    pub fn type_of(&self, name: &str) -> Option<String> {
        self.look_up(name).map(|s| s.type_of.clone())
    }

    pub fn index_of(&self, name: &str) -> Option<usize> {
        self.look_up(name).map(|s| s.index)
    }
}
