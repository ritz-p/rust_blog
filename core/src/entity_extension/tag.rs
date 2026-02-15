use garde::Validate;

#[derive(Validate, Debug)]
#[allow(dead_code)]
pub struct TagValidator {
    #[garde(length(utf16, min = 1, max = 50))]
    pub name: String,
    #[garde(length(utf16, min = 1, max = 50))]
    pub slug: String,
}
