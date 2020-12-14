use syn::{Item, Attribute, parse_quote};

pub fn custom_derive(items: &mut Vec<Item>) {
    for i in items.iter_mut() {
        if let Item::Enum(e) = i {
            let a:Attribute = parse_quote!(
                #[derive(AsRefStr)]
            );
            e.attrs.push(a);
        }
    }

}