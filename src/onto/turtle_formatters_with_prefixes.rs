use rio_api::formatter::TriplesFormatter;
use rio_api::model::*;
use std::collections::HashMap;
use std::io;
use std::io::Write;

#[derive(Copy, Clone)]
enum NamedOrBlankNodeType {
    NamedNode,
    BlankNode,
}

impl NamedOrBlankNodeType {
    fn with_value<'a>(&self, value: &'a str) -> NamedOrBlankNode<'a> {
        match self {
            NamedOrBlankNodeType::NamedNode => NamedNode {
                iri: value,
            }
            .into(),
            NamedOrBlankNodeType::BlankNode => BlankNode {
                id: value,
            }
            .into(),
        }
    }
}

//////////////////////////////////////////////////////////////////////////////////////

pub struct TurtleFormatterWithPrefixes<W: Write> {
    write: W,
    current_subject: String,
    current_subject_type: Option<NamedOrBlankNodeType>,
    current_predicate: String,
}

impl<W: Write> TurtleFormatterWithPrefixes<W> {
    /// Builds a new formatter from a `Write` implementation
    pub fn new(write: W, prefixes: &HashMap<String, String>) -> Self {
        let mut f = TurtleFormatterWithPrefixes {
            write,
            current_subject: String::default(),
            current_subject_type: None,
            current_predicate: String::default(),
        };
        f.write_prefixes(prefixes).unwrap_or_default();
        f
    }

    pub fn write_prefixes(&mut self, prefixes: &HashMap<String, String>) -> Result<(), io::Error> {
        let mut keys: Vec<&String> = prefixes.keys().collect();
        keys.sort();
        for prefix in keys.iter() {
            writeln!(self.write, "@prefix {}: <{}> .", prefix, prefixes.get(prefix.to_owned()).unwrap())?;
        }
        writeln!(self.write)?;
        Ok(())
    }

    /// Finishes to write and returns the underlying `Write`
    pub fn finish(mut self) -> Result<W, io::Error> {
        if self.current_subject_type.is_some() {
            writeln!(self.write, " .")?;
        }
        Ok(self.write)
    }
}

impl<W: Write> TriplesFormatter for TurtleFormatterWithPrefixes<W> {
    type Error = io::Error;

    fn format(&mut self, triple: &Triple<'_>) -> Result<(), io::Error> {
        let s = match triple.subject {
            NamedOrBlankNode::NamedNode(n) => n.iri,
            NamedOrBlankNode::BlankNode(n) => n.id,
        };

        if let Some(current_subject_type) = self.current_subject_type {
            let current_subject = current_subject_type.with_value(&self.current_subject);
            if current_subject == triple.subject {
                if self.current_predicate == *triple.predicate.iri {
                    write!(self.write, ", ")?;
                } else {
                    write!(self.write, " ;\n  {} ", triple.predicate.iri)?;
                }
            } else {
                write!(self.write, " .\n\n{} \n  {} ", &s, triple.predicate.iri)?;
            }
        } else {
            write!(self.write, "{} \n  {} ", &s, triple.predicate.iri)?;
        }
        fmt_object(&triple.object, &mut self.write)?;

        self.current_subject.clear();
        match triple.subject {
            NamedOrBlankNode::NamedNode(n) => {
                self.current_subject = n.iri.to_owned();
                self.current_subject_type = Some(NamedOrBlankNodeType::NamedNode);
            }
            NamedOrBlankNode::BlankNode(n) => {
                self.current_subject.push_str(n.id);
                self.current_subject_type = Some(NamedOrBlankNodeType::BlankNode);
            }
        }
        self.current_predicate.clear();
        self.current_predicate.push_str(triple.predicate.iri);

        Ok(())
    }
}

fn escape(s: &str) -> impl Iterator<Item = char> + '_ {
    s.chars().flat_map(EscapeRDF::new)
}

/// A customized version of EscapeDefault of the Rust standard library
struct EscapeRDF {
    state: EscapeRdfState,
}

enum EscapeRdfState {
    Done,
    Char(char),
    Backslash(char),
}

impl EscapeRDF {
    fn new(c: char) -> Self {
        Self {
            state: match c {
                '\n' => EscapeRdfState::Backslash('n'),
                '\r' => EscapeRdfState::Backslash('r'),
                '"' => EscapeRdfState::Backslash('"'),
                '\\' => EscapeRdfState::Backslash('\\'),
                c => EscapeRdfState::Char(c),
            },
        }
    }
}

impl Iterator for EscapeRDF {
    type Item = char;

    fn next(&mut self) -> Option<char> {
        match self.state {
            EscapeRdfState::Backslash(c) => {
                self.state = EscapeRdfState::Char(c);
                Some('\\')
            }
            EscapeRdfState::Char(c) => {
                self.state = EscapeRdfState::Done;
                Some(c)
            }
            EscapeRdfState::Done => None,
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let n = self.len();
        (n, Some(n))
    }

    fn count(self) -> usize {
        self.len()
    }
}

impl ExactSizeIterator for EscapeRDF {
    fn len(&self) -> usize {
        match self.state {
            EscapeRdfState::Done => 0,
            EscapeRdfState::Char(_) => 1,
            EscapeRdfState::Backslash(_) => 2,
        }
    }
}

fn fmt_object(o: &Term, f: &mut dyn Write) -> Result<(), io::Error> {
    match o {
        Term::NamedNode(n) => {
            f.write_all(n.iri.as_bytes())?;
        }
        Term::BlankNode(n) => {
            f.write_all(n.id.as_bytes())?;
        }
        Term::Literal(v) => match v {
            Literal::Simple {
                value,
            } => {
                f.write_all(b"\"")?;
                escape(value).try_for_each(|c| write!(f, "{}", c))?;
                f.write_all(b"\"")?;
            }
            Literal::LanguageTaggedString {
                value,
                language,
            } => {
                f.write_all(b"\"")?;
                escape(value).try_for_each(|c| write!(f, "{}", c))?;
                write!(f, "\"@{}", language)?;
            }
            Literal::Typed {
                value,
                datatype,
            } => {
                f.write_all(b"\"")?;
                escape(value).try_for_each(|c| write!(f, "{}", c))?;
                write!(f, "\"^^{}", datatype.iri)?;
            }
        },
    }
    Ok(())
}
