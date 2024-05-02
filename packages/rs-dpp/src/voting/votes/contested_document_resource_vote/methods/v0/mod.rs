use crate::prelude::DataContract;
use crate::ProtocolError;

pub trait ContestedDocumentResourceVoteMethodsV0 {
    fn index_path(&self, contract: &DataContract) -> Result<Vec<Vec<u8>>, ProtocolError>;
}