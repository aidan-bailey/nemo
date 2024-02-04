//! The writer for RDF files.

use nemo_physical::datavalues::{AnyDataValue, DataValue, ValueDomain};
use rio_api::{
    formatter::TriplesFormatter,
    model::{BlankNode, Literal, NamedNode, Subject, Term, Triple},
};
use rio_turtle::{NTriplesFormatter, TurtleFormatter};
use rio_xml::RdfXmlFormatter;
use std::io::Write;

use super::types::TableWriter;
use crate::{error::Error, model::RdfVariant};

/// Private struct to record the type of an RDF term that
/// is to be created on demand.
#[derive(Debug, Default)]
enum RdfTermType {
    #[default]
    Iri,
    BNode,
    TypedLiteral,
    LangString,
    SimpleStringLiteral,
}

/// Struct to store information of one quad (or triple) for export.
/// This is necessary since all RIO RDF term implementations use `&str`
/// pointers internally, that must be owned elsewhere.
#[derive(Debug, Default)]
struct QuadBuffer {
    graph: String,
    subject: String,
    predicate: String,
    object_part1: String,
    object_part2: String,
    object_type: RdfTermType,
}
impl<'a> QuadBuffer {
    fn subject(&'a self) -> Subject<'a> {
        Subject::NamedNode(NamedNode {
            iri: &self.subject.as_str(),
        })
    }

    fn predicate(&'a self) -> NamedNode<'a> {
        NamedNode {
            iri: &&self.predicate.as_str(),
        }
    }

    fn object(&'a self) -> Term<'a> {
        match self.object_type {
            RdfTermType::Iri => Term::NamedNode(NamedNode {
                iri: &self.object_part1.as_str(),
            }),
            RdfTermType::BNode => Term::BlankNode(BlankNode {
                id: &self.object_part1.as_str(),
            }),
            RdfTermType::TypedLiteral => Term::Literal(Literal::Typed {
                value: &self.object_part1.as_str(),
                datatype: NamedNode {
                    iri: &self.object_part2.as_str(),
                },
            }),
            RdfTermType::LangString => Term::Literal(Literal::LanguageTaggedString {
                value: &self.object_part1.as_str(),
                language: &self.object_part2.as_str(),
            }),
            RdfTermType::SimpleStringLiteral => Term::Literal(Literal::Simple {
                value: &self.object_part1.as_str(),
            }),
        }
    }

    fn set_subject_from_datavalue(&mut self, datavalue: &AnyDataValue) -> bool {
        match datavalue.value_domain() {
            ValueDomain::Iri => {
                self.subject = datavalue.to_iri_unchecked();
                return true;
            }
            ValueDomain::Null => todo!(),
            _ => false,
        }
    }

    fn set_predicate_from_datavalue(&mut self, datavalue: &AnyDataValue) -> bool {
        match datavalue.value_domain() {
            ValueDomain::Iri => {
                self.predicate = datavalue.to_iri_unchecked();
                return true;
            }
            _ => false,
        }
    }

    fn set_object_from_datavalue(&mut self, datavalue: &AnyDataValue) -> bool {
        match datavalue.value_domain() {
            ValueDomain::String => {
                self.object_type = RdfTermType::SimpleStringLiteral;
                self.object_part1 = datavalue.to_string_unchecked();
            }
            ValueDomain::LanguageTaggedString => {
                self.object_type = RdfTermType::LangString;
                (self.object_part1, self.object_part2) =
                    datavalue.to_language_tagged_string_unchecked();
            }
            ValueDomain::Iri => {
                self.object_type = RdfTermType::Iri;
                self.object_part1 = datavalue.to_iri_unchecked();
            }
            ValueDomain::Float
            | ValueDomain::Double
            | ValueDomain::UnsignedLong
            | ValueDomain::NonNegativeLong
            | ValueDomain::UnsignedInt
            | ValueDomain::NonNegativeInt
            | ValueDomain::Long
            | ValueDomain::Int
            | ValueDomain::Boolean
            | ValueDomain::Other => {
                self.object_type = RdfTermType::TypedLiteral;
                self.object_part1 = datavalue.lexical_value();
                self.object_part2 = datavalue.datatype_iri();
            }
            ValueDomain::Tuple => {
                return false;
            }
            ValueDomain::Map => {
                return false;
            }
            ValueDomain::Null => {
                self.object_type = RdfTermType::BNode;
                // TODO: not supported yet
                return false;
            }
        }
        true
    }
}

/// A writer object for writing RDF files.
pub(super) struct RdfWriter {
    writer: Box<dyn Write>,
    variant: RdfVariant,
    // value_formats: Vec<DsvValueFormat>,
}

impl RdfWriter {
    pub(super) fn new(
        writer: Box<dyn Write>,
        variant: RdfVariant,
        //value_formats: Vec<DsvValueFormat>,
    ) -> Self {
        RdfWriter {
            writer: writer,
            variant: variant,
            // value_formats: value_formats,
        }
    }

    fn export_triples<'a, Formatter>(
        self,
        table: Box<dyn Iterator<Item = Vec<AnyDataValue>> + 'a>,
        make_formatter: impl Fn(Box<dyn Write>) -> std::io::Result<Formatter>,
        finish_formatter: impl Fn(Formatter) -> (),
    ) -> Result<(), Error>
    where
        Formatter: TriplesFormatter,
    {
        // let serializers: Vec<DataValueSerializerFunction> = self
        //     .value_formats
        //     .iter()
        //     .map(|vf| vf.data_value_serializer_function())
        //     .collect();
        // let skip: Vec<bool> = self
        //     .value_formats
        //     .iter()
        //     .map(|vf| *vf == DsvValueFormat::SKIP)
        //     .collect();

        let mut formatter = make_formatter(self.writer)?;

        let mut buffer: QuadBuffer = Default::default();

        for record in table {
            assert_eq!(record.len(), 3);

            if !buffer.set_subject_from_datavalue(&record[0]) {
                continue;
            }
            if !buffer.set_predicate_from_datavalue(&record[1]) {
                continue;
            }
            if !buffer.set_object_from_datavalue(&record[2]) {
                continue;
            }
            if let Err(e) = formatter.format(&Triple {
                subject: buffer.subject(),
                predicate: buffer.predicate(),
                object: buffer.object(),
            }) {
                log::info!("failed to write triple: {e}");
            }
        }
        //let _ = formatter.finish();
        finish_formatter(formatter);

        Ok(())
    }
}

impl TableWriter for RdfWriter {
    fn export_table_data<'a>(
        self: Box<Self>,
        table: Box<dyn Iterator<Item = Vec<AnyDataValue>> + 'a>,
    ) -> Result<(), Error> {
        match self.variant {
            RdfVariant::NTriples => self.export_triples(
                table,
                |write| Ok(NTriplesFormatter::new(write)),
                |f| {
                    let _ = f.finish();
                },
            ),
            RdfVariant::NQuads => todo!(),
            RdfVariant::Turtle => self.export_triples(
                table,
                |write| Ok(TurtleFormatter::new(write)),
                |f| {
                    let _ = f.finish();
                },
            ),
            RdfVariant::RDFXML => self.export_triples(
                table,
                |write| RdfXmlFormatter::new(write),
                |f| {
                    let _ = f.finish();
                },
            ),
            RdfVariant::TriG => todo!(),
            RdfVariant::Unspecified => unreachable!(
                "the writer should not be instantiated with unknown format by the handler"
            ),
        }
    }
}

impl std::fmt::Debug for RdfWriter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RdfWriter")
            .field("write", &"<unspecified std::io::Read>")
            .finish()
    }
}
