pub struct LabelGenerator {
    while_index: usize,
    if_index: usize,
}

impl LabelGenerator {
    /// Создает новый генератор меток с нулевыми счетчиками
    pub fn new() -> Self {
        LabelGenerator {
            while_index: 0,
            if_index: 0,
        }
    }

    /// Генерирует пару меток для цикла: (WHILE_EXP, WHILE_END)
    pub fn next_while(&mut self) -> (String, String) {
        let idx = self.while_index;
        self.while_index += 1;
        (format!("WHILE_EXP{}", idx), format!("WHILE_END{}", idx))
    }

    /// Генерирует пару меток для условия: (IF_TRUE, IF_FALSE)
    /// Если нужен блок else, то третья метка (IF_END) генерируется как IF_FALSE для текущего шага
    pub fn next_if(&mut self) -> (String, String, String) {
        let idx = self.if_index;
        self.if_index += 1;
        (
            format!("IF_TRUE{}", idx),
            format!("IF_FALSE{}", idx),
            format!("IF_END{}", idx),
        )
    }
}

