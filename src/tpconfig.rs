use std::collections::HashSet;
use std::sync::LazyLock;
use tantivy::schema::{Field, STORED, STRING, Schema, TEXT};

pub type SourceName = &'static str;

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum SourceKind {
    Disabled,
    PravneOsebe,
    FizicneOsebe,
    PoslovniRegisterSlovenije,
}

type Position = (usize, usize);

#[derive(Debug)]
pub struct SourceConfig {
    pub name: SourceName,
    pub kind: SourceKind,
    pub source_url: &'static str,
    pub zip_file_path: Option<&'static str>,
    pub data_path: Option<&'static str>,
    pub index_path: Option<&'static str>,
    pub schema: fn() -> Option<&'static (Schema, HashSet<(Field, Position)>)>,
}

static PRAVNE_OSEBE_SCHEMA: LazyLock<(Schema, HashSet<(Field, Position)>)> = LazyLock::new(|| {
    let mut schema_builder = Schema::builder();
    let vat_id = (
        schema_builder.add_text_field("vat_id", STRING | STORED),
        (4, 12),
    );
    let company_id = (
        schema_builder.add_text_field("company_id", STRING | STORED),
        (13, 23),
    );
    let company_name = (
        schema_builder.add_text_field("company_name", TEXT | STORED),
        (42, 143),
    );
    let address = (
        schema_builder.add_text_field("address", TEXT | STORED),
        (143, 257),
    );

    (
        schema_builder.build(),
        HashSet::from([vat_id, company_id, company_name, address]),
    )
});

static FIZICNE_OSEBE_SCHEMA: LazyLock<(Schema, HashSet<(Field, Position)>)> = LazyLock::new(|| {
    let mut schema_builder = Schema::builder();
    let vat_id = (
        schema_builder.add_text_field("vat_id", STRING | STORED),
        (2, 10),
    );
    let name = (
        schema_builder.add_text_field("name", TEXT | STORED),
        (11, 72),
    );
    let address = (
        schema_builder.add_text_field("address", TEXT | STORED),
        (72, 184),
    );
    (
        schema_builder.build(),
        HashSet::from([vat_id, name, address]),
    )
});

pub static CONFIG: [SourceConfig; 3] = [
    SourceConfig {
        name: "Pravne Osebe",
        kind: SourceKind::PravneOsebe,
        source_url: "https://fu.gov.si/fileadmin/prenosi/DURS_zavezanci_PO.zip",
        zip_file_path: Some("DURS_zavezanci_PO.txt"),
        data_path: Some("pravne_osebe.zip"),
        index_path: Some("pravne_osebe"),
        schema: || Some(&PRAVNE_OSEBE_SCHEMA),
    },
    SourceConfig {
        name: "Fizične osebe",
        kind: SourceKind::FizicneOsebe,
        source_url: "https://fu.gov.si/fileadmin/prenosi/DURS_zavezanci_FO.zip",
        zip_file_path: Some("DURS_zavezanci_FO.txt"),
        data_path: Some("fizicne_osebe.zip"),
        index_path: Some("fizicne_osebe"),
        schema: || Some(&FIZICNE_OSEBE_SCHEMA),
    },
    SourceConfig {
        name: "Fizične osebe (dejavnosti)",
        kind: SourceKind::Disabled,
        source_url: "https://fu.gov.si/fileadmin/prenosi/DURS_zavezanci_DEJ.zip",
        zip_file_path: None,
        data_path: Some("fizicne_osebe_dej.zip"),
        index_path: Some("fizicne_osebe_dej"),
        schema: || None,
    },
    /*
    SourceConfig {
        name: "Poslovni Register Slovenije",
        kind: SourceKind::Disabled,
        source_url: "https://podatki.gov.si/dataset/poslovni-register-slovenije",
        zip_file_path: None,
        data_path: Some("poslovni_register_slovenije.zip"),
        index_path: Some("poslovni_register_slovenije"),
        schema: || None,
    }, */
];
