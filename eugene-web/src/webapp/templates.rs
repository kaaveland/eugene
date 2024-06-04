use handlebars::Handlebars;
use once_cell::sync::Lazy;

static HBARS: Lazy<Handlebars> = Lazy::new(|| {
    let mut handlebars = Handlebars::new();
    handlebars.set_strict_mode(true);
    handlebars
        .register_template_string("index", include_str!("templates/index.hbs"))
        .expect("Failed to register index template");
    handlebars
        .register_template_string("base", include_str!("templates/base.hbs"))
        .expect("Failed to register base template");
    handlebars
        .register_template_string("lint", include_str!("templates/lint.hbs"))
        .expect("Failed to register lint template");
    handlebars
});

pub fn handlebars() -> &'static Handlebars<'static> {
    &HBARS
}
