use crate::data_contract::accessors::v0::DataContractV0Getters;
use crate::data_contract::document_type::accessors::DocumentTypeV0Getters;
use crate::prelude::DataContract;
use crate::ProtocolError;
use crate::voting::votes::contested_document_resource_vote::ContestedDocumentResourceVote;
use crate::voting::votes::contested_document_resource_vote::methods::v0::ContestedDocumentResourceVoteMethodsV0;
use crate::voting::votes::contested_document_resource_vote::accessors::v0::ContestedDocumentResourceVoteGettersV0;

mod v0;

impl ContestedDocumentResourceVoteMethodsV0 for ContestedDocumentResourceVote {
    fn index_path(&self, contract: &DataContract) -> Result<Vec<Vec<u8>>, ProtocolError> {
        let vote_poll = self.vote_poll();
        let document_type = contract.document_type_for_name(vote_poll.document_type_name.as_str())?;
        let index = document_type.indices().get(&vote_poll.index_name).ok_or(ProtocolError::);
    }
}