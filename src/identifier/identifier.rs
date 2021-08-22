use super::errors::IdentifierError;

pub struct Identifier {}

impl Identifier {
    fn new(buffer: &Vec<u8>) -> Result<Identifier, IdentifierError> {
        if buffer.len() != 32 {
            return Result::Err(IdentifierError);
        }

        return Result::Ok()
    }

    fn from(value: &str, encoding: Option<&str>) {}
}