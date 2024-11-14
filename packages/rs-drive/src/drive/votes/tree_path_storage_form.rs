use dpp::ProtocolError;

/// A trait to figure out vote info from a tree path
pub trait TreePathStorageForm {
    /// Construction of the resource vote from the tree oath
    fn try_from_tree_path(path: Vec<Vec<u8>>) -> Result<Self, ProtocolError>
    where
        Self: Sized;
}
