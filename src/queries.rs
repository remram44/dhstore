pub struct Query {
    key: String,
    value: Comparison,
}

pub enum Comparison {
    Equal(String),
    Like(String),
    Range(Option<i64>, Option<i64>),
    And(Vec<Comparison>),
    Or(Vec<Comparison>),
}

pub struct Path {
    root: ID,
    components: Vec<PathComponent>,
}

pub enum PathComponent {
    Id(ID),
    Query(Query),
}
