use std::collections::HashSet;
use std::sync::LazyLock;
use tantivy::schema::{Field, STORED, STRING, Schema, TEXT};

pub type SourceName = &'static str;

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum SourceKind {
    Disabled,
    PravneOsebe,
    FizicneOsebe,
}

#[derive(Debug)]
pub struct SourceConfig {
    pub name: SourceName,
    pub kind: SourceKind,
    pub source_url: &'static str,
    pub data_path: Option<&'static str>,
    pub index_path: Option<&'static str>,
    pub schema: fn() -> Option<&'static (Schema, HashSet<Field>)>,
}

static PRAVNE_OSEBE_SCHEMA: LazyLock<(Schema, HashSet<Field>)> = LazyLock::new(|| {
    let mut schema_builder = Schema::builder();
    let vat_field = schema_builder.add_text_field("vat_id", STRING | STORED);
    let company_id = schema_builder.add_text_field("company_id", STRING | STORED);
    let company_name = schema_builder.add_text_field("company_name", TEXT | STORED);

    (
        schema_builder.build(),
        HashSet::from([vat_field, company_id]),
    )
});

pub static CONFIG: [SourceConfig; 3] = [
    SourceConfig {
        name: "Pravne Osebe",
        kind: SourceKind::PravneOsebe,
        source_url: "https://fu.gov.si/fileadmin/prenosi/DURS_zavezanci_PO.zip",
        data_path: Some("pravne_osebe.zip"),
        index_path: Some("pravne_osebe"),
        schema: || Some(&PRAVNE_OSEBE_SCHEMA),
    },
    SourceConfig {
        name: "Fizične osebe",
        kind: SourceKind::FizicneOsebe,
        source_url: "https://fu.gov.si/fileadmin/prenosi/DURS_zavezanci_FO.zip",
        data_path: Some("fizicne_osebe.zip"),
        index_path: Some("fizicne_osebe"),
        schema: || None,
    },
    SourceConfig {
        name: "Fizične osebe (dejavnosti)",
        kind: SourceKind::Disabled,
        source_url: "https://fu.gov.si/fileadmin/prenosi/DURS_zavezanci_DEJ.zip",
        data_path: Some("fizicne_osebe_dej.zip"),
        index_path: Some("fizicne_osebe_dej"),
        schema: || None,
    },
];
