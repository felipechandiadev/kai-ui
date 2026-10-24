use kai::icons::Icon;
use kai::router::SectionMeta;

pub const SECTION: SectionMeta = SectionMeta {
    title: "Clientes",
    icon: Icon::Users,
    order: 10,
    group: Some("Datos"),
};
