//! Bourbaki Kernel: Minimal CIC Translation Layer and Proof Term Extractor.

pub mod cic;
pub mod extractor;

pub use cic::{Level, Sort, Term};
pub use extractor::{ExtractionError, TermExtractor};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_smoke_kernel() {
        let sort = Sort::Prop;
        let term = Term::Sort(sort);
        assert_eq!(term, Term::Sort(Sort::Prop));
    }
}
