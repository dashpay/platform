//! # Group Actions Module
//!
//! This module provides functionality for group-based actions and collaborative operations

use crate::sdk::WasmSdk;
use crate::dapi_client::{DapiClient, DapiClientConfig};
use dpp::prelude::Identifier;
use dpp::state_transition::{StateTransition, batch_transition::{BatchTransition, BatchTransitionV0}};
use dpp::serialization::PlatformSerializable;
use js_sys::{Array, Date, Object, Reflect};
use wasm_bindgen::prelude::*;

/// Group types
#[wasm_bindgen]
#[derive(Clone, Debug)]
pub enum GroupType {
    Multisig,
    DAO,
    Committee,
    Custom,
}

/// Group member role
#[wasm_bindgen]
#[derive(Clone, Debug)]
pub enum MemberRole {
    Owner,
    Admin,
    Member,
    Observer,
}

/// Group information
#[wasm_bindgen]
pub struct Group {
    id: String,
    name: String,
    description: String,
    group_type: GroupType,
    created_at: u64,
    member_count: u32,
    threshold: u32,
    active: bool,
}

#[wasm_bindgen]
impl Group {
    /// Get group ID
    #[wasm_bindgen(getter)]
    pub fn id(&self) -> String {
        self.id.clone()
    }

    /// Get group name
    #[wasm_bindgen(getter)]
    pub fn name(&self) -> String {
        self.name.clone()
    }

    /// Get group description
    #[wasm_bindgen(getter)]
    pub fn description(&self) -> String {
        self.description.clone()
    }

    /// Get group type
    #[wasm_bindgen(getter, js_name = groupType)]
    pub fn group_type_str(&self) -> String {
        match self.group_type {
            GroupType::Multisig => "multisig".to_string(),
            GroupType::DAO => "dao".to_string(),
            GroupType::Committee => "committee".to_string(),
            GroupType::Custom => "custom".to_string(),
        }
    }

    /// Get creation timestamp
    #[wasm_bindgen(getter, js_name = createdAt)]
    pub fn created_at(&self) -> u64 {
        self.created_at
    }

    /// Get member count
    #[wasm_bindgen(getter, js_name = memberCount)]
    pub fn member_count(&self) -> u32 {
        self.member_count
    }

    /// Get threshold for actions
    #[wasm_bindgen(getter)]
    pub fn threshold(&self) -> u32 {
        self.threshold
    }

    /// Check if group is active
    #[wasm_bindgen(getter)]
    pub fn active(&self) -> bool {
        self.active
    }

    /// Convert to JavaScript object
    #[wasm_bindgen(js_name = toObject)]
    pub fn to_object(&self) -> Result<JsValue, JsError> {
        let obj = Object::new();
        Reflect::set(&obj, &"id".into(), &self.id.clone().into())
            .map_err(|_| JsError::new("Failed to set id"))?;
        Reflect::set(&obj, &"name".into(), &self.name.clone().into())
            .map_err(|_| JsError::new("Failed to set name"))?;
        Reflect::set(&obj, &"description".into(), &self.description.clone().into())
            .map_err(|_| JsError::new("Failed to set description"))?;
        Reflect::set(&obj, &"groupType".into(), &self.group_type_str().into())
            .map_err(|_| JsError::new("Failed to set group type"))?;
        Reflect::set(&obj, &"createdAt".into(), &self.created_at.into())
            .map_err(|_| JsError::new("Failed to set created at"))?;
        Reflect::set(&obj, &"memberCount".into(), &self.member_count.into())
            .map_err(|_| JsError::new("Failed to set member count"))?;
        Reflect::set(&obj, &"threshold".into(), &self.threshold.into())
            .map_err(|_| JsError::new("Failed to set threshold"))?;
        Reflect::set(&obj, &"active".into(), &self.active.into())
            .map_err(|_| JsError::new("Failed to set active"))?;
        Ok(obj.into())
    }
}

/// Group member information
#[wasm_bindgen]
pub struct GroupMember {
    identity_id: String,
    role: MemberRole,
    joined_at: u64,
    permissions: Vec<String>,
}

#[wasm_bindgen]
impl GroupMember {
    /// Get member identity ID
    #[wasm_bindgen(getter, js_name = identityId)]
    pub fn identity_id(&self) -> String {
        self.identity_id.clone()
    }

    /// Get member role
    #[wasm_bindgen(getter)]
    pub fn role(&self) -> String {
        match self.role {
            MemberRole::Owner => "owner".to_string(),
            MemberRole::Admin => "admin".to_string(),
            MemberRole::Member => "member".to_string(),
            MemberRole::Observer => "observer".to_string(),
        }
    }

    /// Get join timestamp
    #[wasm_bindgen(getter, js_name = joinedAt)]
    pub fn joined_at(&self) -> u64 {
        self.joined_at
    }

    /// Get permissions
    #[wasm_bindgen(getter)]
    pub fn permissions(&self) -> Array {
        let arr = Array::new();
        for perm in &self.permissions {
            arr.push(&perm.into());
        }
        arr
    }

    /// Check if member has permission
    #[wasm_bindgen(js_name = hasPermission)]
    pub fn has_permission(&self, permission: &str) -> bool {
        self.permissions.contains(&permission.to_string())
    }
}

/// Group action proposal
#[wasm_bindgen]
pub struct GroupProposal {
    id: String,
    group_id: String,
    proposer_id: String,
    title: String,
    description: String,
    action_type: String,
    action_data: Vec<u8>,
    created_at: u64,
    expires_at: u64,
    approvals: u32,
    rejections: u32,
    executed: bool,
}

#[wasm_bindgen]
impl GroupProposal {
    /// Get proposal ID
    #[wasm_bindgen(getter)]
    pub fn id(&self) -> String {
        self.id.clone()
    }

    /// Get group ID
    #[wasm_bindgen(getter, js_name = groupId)]
    pub fn group_id(&self) -> String {
        self.group_id.clone()
    }

    /// Get proposer ID
    #[wasm_bindgen(getter, js_name = proposerId)]
    pub fn proposer_id(&self) -> String {
        self.proposer_id.clone()
    }

    /// Get title
    #[wasm_bindgen(getter)]
    pub fn title(&self) -> String {
        self.title.clone()
    }

    /// Get description
    #[wasm_bindgen(getter)]
    pub fn description(&self) -> String {
        self.description.clone()
    }

    /// Get action type
    #[wasm_bindgen(getter, js_name = actionType)]
    pub fn action_type(&self) -> String {
        self.action_type.clone()
    }

    /// Get action data
    #[wasm_bindgen(getter, js_name = actionData)]
    pub fn action_data(&self) -> Vec<u8> {
        self.action_data.clone()
    }

    /// Get creation timestamp
    #[wasm_bindgen(getter, js_name = createdAt)]
    pub fn created_at(&self) -> u64 {
        self.created_at
    }

    /// Get expiration timestamp
    #[wasm_bindgen(getter, js_name = expiresAt)]
    pub fn expires_at(&self) -> u64 {
        self.expires_at
    }

    /// Get approval count
    #[wasm_bindgen(getter)]
    pub fn approvals(&self) -> u32 {
        self.approvals
    }

    /// Get rejection count
    #[wasm_bindgen(getter)]
    pub fn rejections(&self) -> u32 {
        self.rejections
    }

    /// Check if executed
    #[wasm_bindgen(getter)]
    pub fn executed(&self) -> bool {
        self.executed
    }

    /// Check if proposal is active
    #[wasm_bindgen(js_name = isActive)]
    pub fn is_active(&self) -> bool {
        !self.executed && (Date::now() as u64) < self.expires_at
    }

    /// Check if proposal is expired
    #[wasm_bindgen(js_name = isExpired)]
    pub fn is_expired(&self) -> bool {
        (Date::now() as u64) >= self.expires_at
    }

    /// Convert to JavaScript object
    #[wasm_bindgen(js_name = toObject)]
    pub fn to_object(&self) -> Result<JsValue, JsError> {
        let obj = Object::new();
        Reflect::set(&obj, &"id".into(), &self.id.clone().into())
            .map_err(|_| JsError::new("Failed to set id"))?;
        Reflect::set(&obj, &"groupId".into(), &self.group_id.clone().into())
            .map_err(|_| JsError::new("Failed to set group id"))?;
        Reflect::set(&obj, &"proposerId".into(), &self.proposer_id.clone().into())
            .map_err(|_| JsError::new("Failed to set proposer id"))?;
        Reflect::set(&obj, &"title".into(), &self.title.clone().into())
            .map_err(|_| JsError::new("Failed to set title"))?;
        Reflect::set(&obj, &"description".into(), &self.description.clone().into())
            .map_err(|_| JsError::new("Failed to set description"))?;
        Reflect::set(&obj, &"actionType".into(), &self.action_type.clone().into())
            .map_err(|_| JsError::new("Failed to set action type"))?;
        Reflect::set(&obj, &"createdAt".into(), &self.created_at.into())
            .map_err(|_| JsError::new("Failed to set created at"))?;
        Reflect::set(&obj, &"expiresAt".into(), &self.expires_at.into())
            .map_err(|_| JsError::new("Failed to set expires at"))?;
        Reflect::set(&obj, &"approvals".into(), &self.approvals.into())
            .map_err(|_| JsError::new("Failed to set approvals"))?;
        Reflect::set(&obj, &"rejections".into(), &self.rejections.into())
            .map_err(|_| JsError::new("Failed to set rejections"))?;
        Reflect::set(&obj, &"executed".into(), &self.executed.into())
            .map_err(|_| JsError::new("Failed to set executed"))?;
        Ok(obj.into())
    }
}

/// Create a new group
#[wasm_bindgen(js_name = createGroup)]
pub fn create_group(
    creator_id: &str,
    name: &str,
    description: &str,
    group_type: &str,
    threshold: u32,
    initial_members: Array,
    identity_nonce: u64,
    signature_public_key_id: u32,
) -> Result<Vec<u8>, JsError> {
    let _creator = Identifier::from_string(
        creator_id,
        platform_value::string_encoding::Encoding::Base58,
    )
    .map_err(|e| JsError::new(&format!("Invalid creator ID: {}", e)))?;

    // Parse group type
    let _group_type = match group_type.to_lowercase().as_str() {
        "multisig" => GroupType::Multisig,
        "dao" => GroupType::DAO,
        "committee" => GroupType::Committee,
        _ => GroupType::Custom,
    };

    // Convert members array
    let mut members = Vec::new();
    for i in 0..initial_members.length() {
        if let Some(member) = initial_members.get(i).as_string() {
            members.push(member);
        }
    }

    // Create group document for the state transition
    // This would create a document in a groups data contract
    let group_id = format!("group_{}_{}_{}", creator_id, name, Date::now() as u64);
    let group_doc = Object::new();
    
    // Set document properties
    Reflect::set(&group_doc, &"$id".into(), &group_id.clone().into())
        .map_err(|_| JsError::new("Failed to set group id"))?;
    Reflect::set(&group_doc, &"$type".into(), &"group".into())
        .map_err(|_| JsError::new("Failed to set document type"))?;
    Reflect::set(&group_doc, &"creatorId".into(), &creator_id.into())
        .map_err(|_| JsError::new("Failed to set creator id"))?;
    Reflect::set(&group_doc, &"name".into(), &name.into())
        .map_err(|_| JsError::new("Failed to set name"))?;
    Reflect::set(&group_doc, &"description".into(), &description.into())
        .map_err(|_| JsError::new("Failed to set description"))?;
    Reflect::set(&group_doc, &"groupType".into(), &group_type.into())
        .map_err(|_| JsError::new("Failed to set group type"))?;
    Reflect::set(&group_doc, &"threshold".into(), &threshold.into())
        .map_err(|_| JsError::new("Failed to set threshold"))?;
    Reflect::set(&group_doc, &"members".into(), &initial_members)
        .map_err(|_| JsError::new("Failed to set members"))?;
    Reflect::set(&group_doc, &"active".into(), &true.into())
        .map_err(|_| JsError::new("Failed to set active status"))?;
    Reflect::set(&group_doc, &"createdAt".into(), &(Date::now() as u64).into())
        .map_err(|_| JsError::new("Failed to set created at"))?;

    // Create a simplified batch transition
    // In production, this would include proper document create transitions
    let batch_transition = BatchTransition::V0(BatchTransitionV0 {
        owner_id: _creator.clone(),
        transitions: vec![], // Document transitions would go here
        user_fee_increase: 0,
        signature_public_key_id: signature_public_key_id as u32,
        signature: Default::default(),
    });

    // Serialize the transition
    StateTransition::Batch(batch_transition)
        .serialize_to_bytes()
        .map_err(|e| JsError::new(&format!("Failed to serialize state transition: {}", e)))
}

/// Add member to group
#[wasm_bindgen(js_name = addGroupMember)]
pub fn add_group_member(
    group_id: &str,
    admin_id: &str,
    new_member_id: &str,
    role: &str,
    permissions: Array,
    identity_nonce: u64,
    signature_public_key_id: u32,
) -> Result<Vec<u8>, JsError> {
    let _group = Identifier::from_string(
        group_id,
        platform_value::string_encoding::Encoding::Base58,
    )
    .map_err(|e| JsError::new(&format!("Invalid group ID: {}", e)))?;

    let _admin = Identifier::from_string(
        admin_id,
        platform_value::string_encoding::Encoding::Base58,
    )
    .map_err(|e| JsError::new(&format!("Invalid admin ID: {}", e)))?;

    let _new_member = Identifier::from_string(
        new_member_id,
        platform_value::string_encoding::Encoding::Base58,
    )
    .map_err(|e| JsError::new(&format!("Invalid new member ID: {}", e)))?;

    // Convert permissions
    let mut perms = Vec::new();
    for i in 0..permissions.length() {
        if let Some(perm) = permissions.get(i).as_string() {
            perms.push(perm);
        }
    }

    // Create member document for the state transition
    let member_id = format!("member_{}_{}", group_id, new_member_id);
    let member_doc = Object::new();
    
    // Set document properties
    Reflect::set(&member_doc, &"$id".into(), &member_id.clone().into())
        .map_err(|_| JsError::new("Failed to set member id"))?;
    Reflect::set(&member_doc, &"$type".into(), &"groupMember".into())
        .map_err(|_| JsError::new("Failed to set document type"))?;
    Reflect::set(&member_doc, &"groupId".into(), &group_id.into())
        .map_err(|_| JsError::new("Failed to set group id"))?;
    Reflect::set(&member_doc, &"identityId".into(), &new_member_id.into())
        .map_err(|_| JsError::new("Failed to set identity id"))?;
    Reflect::set(&member_doc, &"role".into(), &role.into())
        .map_err(|_| JsError::new("Failed to set role"))?;
    Reflect::set(&member_doc, &"permissions".into(), &permissions)
        .map_err(|_| JsError::new("Failed to set permissions"))?;
    Reflect::set(&member_doc, &"addedBy".into(), &admin_id.into())
        .map_err(|_| JsError::new("Failed to set added by"))?;
    Reflect::set(&member_doc, &"joinedAt".into(), &(Date::now() as u64).into())
        .map_err(|_| JsError::new("Failed to set joined at"))?;

    // Create a document create transition
    let documents_to_create = Array::new();
    documents_to_create.push(&member_doc.into());
    
    // Create a simplified batch transition for adding member
    let batch_transition = BatchTransition::V0(BatchTransitionV0 {
        owner_id: _admin.clone(),
        transitions: vec![], // Document create transition would go here
        user_fee_increase: 0,
        signature_public_key_id: signature_public_key_id as u32,
        signature: Default::default(),
    });

    // Serialize the transition
    StateTransition::Batch(batch_transition)
        .serialize_to_bytes()
        .map_err(|e| JsError::new(&format!("Failed to serialize state transition: {}", e)))
}

/// Remove member from group
#[wasm_bindgen(js_name = removeGroupMember)]
pub fn remove_group_member(
    group_id: &str,
    admin_id: &str,
    member_id: &str,
    identity_nonce: u64,
    signature_public_key_id: u32,
) -> Result<Vec<u8>, JsError> {
    let _group = Identifier::from_string(
        group_id,
        platform_value::string_encoding::Encoding::Base58,
    )
    .map_err(|e| JsError::new(&format!("Invalid group ID: {}", e)))?;

    let _admin = Identifier::from_string(
        admin_id,
        platform_value::string_encoding::Encoding::Base58,
    )
    .map_err(|e| JsError::new(&format!("Invalid admin ID: {}", e)))?;

    let _member = Identifier::from_string(
        member_id,
        platform_value::string_encoding::Encoding::Base58,
    )
    .map_err(|e| JsError::new(&format!("Invalid member ID: {}", e)))?;

    // Create a document delete transition for the member
    let member_doc_id = format!("member_{}_{}", group_id, member_id);
    let documents_to_delete = Array::new();
    
    let delete_obj = Object::new();
    Reflect::set(&delete_obj, &"$id".into(), &member_doc_id.into())
        .map_err(|_| JsError::new("Failed to set document id for deletion"))?;
    Reflect::set(&delete_obj, &"$type".into(), &"groupMember".into())
        .map_err(|_| JsError::new("Failed to set document type for deletion"))?;
    
    documents_to_delete.push(&delete_obj.into());
    
    // Create a simplified batch transition for removing member
    let batch_transition = BatchTransition::V0(BatchTransitionV0 {
        owner_id: _admin.clone(),
        transitions: vec![], // Document delete transition would go here
        user_fee_increase: 0,
        signature_public_key_id: signature_public_key_id as u32,
        signature: Default::default(),
    });

    // Serialize the transition
    StateTransition::Batch(batch_transition)
        .serialize_to_bytes()
        .map_err(|e| JsError::new(&format!("Failed to serialize state transition: {}", e)))
}

/// Create a group proposal
#[wasm_bindgen(js_name = createGroupProposal)]
pub fn create_group_proposal(
    group_id: &str,
    proposer_id: &str,
    title: &str,
    description: &str,
    action_type: &str,
    action_data: Vec<u8>,
    duration_hours: u32,
    identity_nonce: u64,
    signature_public_key_id: u32,
) -> Result<Vec<u8>, JsError> {
    let _group = Identifier::from_string(
        group_id,
        platform_value::string_encoding::Encoding::Base58,
    )
    .map_err(|e| JsError::new(&format!("Invalid group ID: {}", e)))?;

    let _proposer = Identifier::from_string(
        proposer_id,
        platform_value::string_encoding::Encoding::Base58,
    )
    .map_err(|e| JsError::new(&format!("Invalid proposer ID: {}", e)))?;

    // Create proposal document for the state transition
    let proposal_id = format!("proposal_{}_{}", group_id, Date::now() as u64);
    let proposal_doc = Object::new();
    
    // Set document properties
    Reflect::set(&proposal_doc, &"$id".into(), &proposal_id.clone().into())
        .map_err(|_| JsError::new("Failed to set proposal id"))?;
    Reflect::set(&proposal_doc, &"$type".into(), &"groupProposal".into())
        .map_err(|_| JsError::new("Failed to set document type"))?;
    Reflect::set(&proposal_doc, &"groupId".into(), &group_id.into())
        .map_err(|_| JsError::new("Failed to set group id"))?;
    Reflect::set(&proposal_doc, &"proposerId".into(), &proposer_id.into())
        .map_err(|_| JsError::new("Failed to set proposer id"))?;
    Reflect::set(&proposal_doc, &"title".into(), &title.into())
        .map_err(|_| JsError::new("Failed to set title"))?;
    Reflect::set(&proposal_doc, &"description".into(), &description.into())
        .map_err(|_| JsError::new("Failed to set description"))?;
    Reflect::set(&proposal_doc, &"actionType".into(), &action_type.into())
        .map_err(|_| JsError::new("Failed to set action type"))?;
    
    // Convert action data to base64 for storage
    use base64::{Engine as _, engine::general_purpose::STANDARD};
    let action_data_b64 = STANDARD.encode(&action_data);
    Reflect::set(&proposal_doc, &"actionData".into(), &action_data_b64.into())
        .map_err(|_| JsError::new("Failed to set action data"))?;
    
    let created_at = Date::now() as u64;
    let expires_at = created_at + (duration_hours as u64 * 3600 * 1000); // Convert hours to milliseconds
    
    Reflect::set(&proposal_doc, &"createdAt".into(), &created_at.into())
        .map_err(|_| JsError::new("Failed to set created at"))?;
    Reflect::set(&proposal_doc, &"expiresAt".into(), &expires_at.into())
        .map_err(|_| JsError::new("Failed to set expires at"))?;
    Reflect::set(&proposal_doc, &"approvals".into(), &0.into())
        .map_err(|_| JsError::new("Failed to set approvals"))?;
    Reflect::set(&proposal_doc, &"rejections".into(), &0.into())
        .map_err(|_| JsError::new("Failed to set rejections"))?;
    Reflect::set(&proposal_doc, &"executed".into(), &false.into())
        .map_err(|_| JsError::new("Failed to set executed"))?;

    // Create a document create transition
    let documents_to_create = Array::new();
    documents_to_create.push(&proposal_doc.into());
    
    // Create a simplified batch transition for creating proposal
    let batch_transition = BatchTransition::V0(BatchTransitionV0 {
        owner_id: _proposer.clone(),
        transitions: vec![], // Document create transition would go here
        user_fee_increase: 0,
        signature_public_key_id: signature_public_key_id as u32,
        signature: Default::default(),
    });

    // Serialize the transition
    StateTransition::Batch(batch_transition)
        .serialize_to_bytes()
        .map_err(|e| JsError::new(&format!("Failed to serialize state transition: {}", e)))
}

/// Vote on group proposal
#[wasm_bindgen(js_name = voteOnProposal)]
pub fn vote_on_proposal(
    proposal_id: &str,
    voter_id: &str,
    approve: bool,
    comment: Option<String>,
    identity_nonce: u64,
    signature_public_key_id: u32,
) -> Result<Vec<u8>, JsError> {
    let _proposal = Identifier::from_string(
        proposal_id,
        platform_value::string_encoding::Encoding::Base58,
    )
    .map_err(|e| JsError::new(&format!("Invalid proposal ID: {}", e)))?;

    let _voter = Identifier::from_string(
        voter_id,
        platform_value::string_encoding::Encoding::Base58,
    )
    .map_err(|e| JsError::new(&format!("Invalid voter ID: {}", e)))?;

    // Create vote document for the state transition
    let vote_id = format!("vote_{}_{}_{}", proposal_id, voter_id, Date::now() as u64);
    let vote_doc = Object::new();
    
    // Set document properties
    Reflect::set(&vote_doc, &"$id".into(), &vote_id.clone().into())
        .map_err(|_| JsError::new("Failed to set vote id"))?;
    Reflect::set(&vote_doc, &"$type".into(), &"proposalVote".into())
        .map_err(|_| JsError::new("Failed to set document type"))?;
    Reflect::set(&vote_doc, &"proposalId".into(), &proposal_id.into())
        .map_err(|_| JsError::new("Failed to set proposal id"))?;
    Reflect::set(&vote_doc, &"voterId".into(), &voter_id.into())
        .map_err(|_| JsError::new("Failed to set voter id"))?;
    Reflect::set(&vote_doc, &"vote".into(), &(if approve { "approve" } else { "reject" }).into())
        .map_err(|_| JsError::new("Failed to set vote"))?;
    Reflect::set(&vote_doc, &"votedAt".into(), &(Date::now() as u64).into())
        .map_err(|_| JsError::new("Failed to set voted at"))?;
    
    if let Some(comment_text) = comment {
        Reflect::set(&vote_doc, &"comment".into(), &comment_text.into())
            .map_err(|_| JsError::new("Failed to set comment"))?;
    }

    // Create a document create transition
    let documents_to_create = Array::new();
    documents_to_create.push(&vote_doc.into());
    
    // Create a simplified batch transition for voting
    let batch_transition = BatchTransition::V0(BatchTransitionV0 {
        owner_id: _voter.clone(),
        transitions: vec![], // Document create transition would go here
        user_fee_increase: 0,
        signature_public_key_id: signature_public_key_id as u32,
        signature: Default::default(),
    });

    // Serialize the transition
    StateTransition::Batch(batch_transition)
        .serialize_to_bytes()
        .map_err(|e| JsError::new(&format!("Failed to serialize state transition: {}", e)))
}

/// Execute approved proposal
#[wasm_bindgen(js_name = executeProposal)]
pub fn execute_proposal(
    proposal_id: &str,
    executor_id: &str,
    identity_nonce: u64,
    signature_public_key_id: u32,
) -> Result<Vec<u8>, JsError> {
    let _proposal = Identifier::from_string(
        proposal_id,
        platform_value::string_encoding::Encoding::Base58,
    )
    .map_err(|e| JsError::new(&format!("Invalid proposal ID: {}", e)))?;

    let _executor = Identifier::from_string(
        executor_id,
        platform_value::string_encoding::Encoding::Base58,
    )
    .map_err(|e| JsError::new(&format!("Invalid executor ID: {}", e)))?;

    // Update proposal document to mark it as executed
    let update_obj = Object::new();
    
    // Document ID to update
    Reflect::set(&update_obj, &"$id".into(), &proposal_id.into())
        .map_err(|_| JsError::new("Failed to set proposal id for update"))?;
    Reflect::set(&update_obj, &"$type".into(), &"groupProposal".into())
        .map_err(|_| JsError::new("Failed to set document type for update"))?;
    
    // Fields to update
    Reflect::set(&update_obj, &"executed".into(), &true.into())
        .map_err(|_| JsError::new("Failed to set executed status"))?;
    Reflect::set(&update_obj, &"executedBy".into(), &executor_id.into())
        .map_err(|_| JsError::new("Failed to set executed by"))?;
    Reflect::set(&update_obj, &"executedAt".into(), &(Date::now() as u64).into())
        .map_err(|_| JsError::new("Failed to set executed at"))?;

    // Create a document update transition
    let documents_to_update = Array::new();
    documents_to_update.push(&update_obj.into());
    
    // Create a simplified batch transition for executing proposal
    let batch_transition = BatchTransition::V0(BatchTransitionV0 {
        owner_id: _executor.clone(),
        transitions: vec![], // Document update transition would go here
        user_fee_increase: 0,
        signature_public_key_id: signature_public_key_id as u32,
        signature: Default::default(),
    });

    // Serialize the transition
    StateTransition::Batch(batch_transition)
        .serialize_to_bytes()
        .map_err(|e| JsError::new(&format!("Failed to serialize state transition: {}", e)))
}

/// Fetch group information
#[wasm_bindgen(js_name = fetchGroup)]
pub async fn fetch_group(
    sdk: &WasmSdk,
    group_id: &str,
) -> Result<Group, JsError> {
    let _sdk = sdk;
    let _identifier = Identifier::from_string(
        group_id,
        platform_value::string_encoding::Encoding::Base58,
    )
    .map_err(|e| JsError::new(&format!("Invalid group ID: {}", e)))?;

    // Fetch group document from platform
    let config = DapiClientConfig::new(sdk.network());
    let client = DapiClient::new(config)?;
    
    // Query for the group document
    let query = Object::new();
    let where_clause = js_sys::Array::new();
    let id_condition = js_sys::Array::of3(
        &"$id".into(),
        &"==".into(),
        &group_id.into()
    );
    where_clause.push(&id_condition);
    
    Reflect::set(&query, &"where".into(), &where_clause)
        .map_err(|_| JsError::new("Failed to set where clause"))?;
    Reflect::set(&query, &"limit".into(), &1.into())
        .map_err(|_| JsError::new("Failed to set limit"))?;
    
    let groups_contract_id = "GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec"; // System groups contract
    let documents = client.get_documents(
        groups_contract_id.to_string(),
        "group".to_string(),
        query.into(),
        JsValue::null(),
        1,
        None,
        false
    ).await?;
    
    // Parse the response
    if let Some(docs_array) = js_sys::Reflect::get(&documents, &"documents".into())
        .map_err(|_| JsError::new("Failed to get documents from response"))?
        .dyn_ref::<js_sys::Array>() {
        if docs_array.length() > 0 {
            let group_doc = docs_array.get(0);
            
            // Extract group properties
            let name = js_sys::Reflect::get(&group_doc, &"name".into())
                .map_err(|_| JsError::new("Failed to get group name"))?
                .as_string()
                .unwrap_or_else(|| "Unknown Group".to_string());
            let description = js_sys::Reflect::get(&group_doc, &"description".into())
                .map_err(|_| JsError::new("Failed to get group description"))?
                .as_string()
                .unwrap_or_else(|| "No description".to_string());
            let group_type_str = js_sys::Reflect::get(&group_doc, &"groupType".into())
                .map_err(|_| JsError::new("Failed to get group type"))?
                .as_string()
                .unwrap_or_else(|| "custom".to_string());
            let created_at = js_sys::Reflect::get(&group_doc, &"createdAt".into())
                .map_err(|_| JsError::new("Failed to get created_at"))?
                .as_f64()
                .unwrap_or(0.0) as u64;
            let member_count = js_sys::Reflect::get(&group_doc, &"members".into())
                .map_err(|_| JsError::new("Failed to get members"))?
                .dyn_ref::<js_sys::Array>()
                .map(|arr| arr.length())
                .unwrap_or(0);
            let threshold = js_sys::Reflect::get(&group_doc, &"threshold".into())
                .map_err(|_| JsError::new("Failed to get threshold"))?
                .as_f64()
                .unwrap_or(1.0) as u32;
            let active = js_sys::Reflect::get(&group_doc, &"active".into())
                .map_err(|_| JsError::new("Failed to get active status"))?
                .as_bool()
                .unwrap_or(true);
            
            let group_type = match group_type_str.as_str() {
                "multisig" => GroupType::Multisig,
                "dao" => GroupType::DAO,
                "committee" => GroupType::Committee,
                _ => GroupType::Custom,
            };
            
            return Ok(Group {
                id: group_id.to_string(),
                name,
                description,
                group_type,
                created_at,
                member_count,
                threshold,
                active,
            });
        }
    }
    
    Err(JsError::new("Group not found"))
}

/// Fetch group members
#[wasm_bindgen(js_name = fetchGroupMembers)]
pub async fn fetch_group_members(
    sdk: &WasmSdk,
    group_id: &str,
) -> Result<Array, JsError> {
    let _sdk = sdk;
    let _identifier = Identifier::from_string(
        group_id,
        platform_value::string_encoding::Encoding::Base58,
    )
    .map_err(|e| JsError::new(&format!("Invalid group ID: {}", e)))?;

    // Fetch group members from platform
    let config = DapiClientConfig::new(sdk.network());
    let client = DapiClient::new(config)?;
    
    // Query for group member documents
    let query = Object::new();
    let where_clause = js_sys::Array::new();
    let group_condition = js_sys::Array::of3(
        &"groupId".into(),
        &"==".into(),
        &group_id.into()
    );
    where_clause.push(&group_condition);
    
    Reflect::set(&query, &"where".into(), &where_clause)
        .map_err(|_| JsError::new("Failed to set where clause"))?;
    Reflect::set(&query, &"limit".into(), &100.into())
        .map_err(|_| JsError::new("Failed to set limit"))?;
    
    let groups_contract_id = "GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec"; // System groups contract
    let documents = client.get_documents(
        groups_contract_id.to_string(),
        "groupMember".to_string(),
        query.into(),
        JsValue::null(),
        100,
        None,
        false
    ).await?;
    
    // Parse and return the members array
    if let Some(docs_array) = js_sys::Reflect::get(&documents, &"documents".into())
        .map_err(|_| JsError::new("Failed to get documents from response"))?
        .dyn_ref::<js_sys::Array>() {
        return Ok(docs_array.clone());
    }
    
    Ok(Array::new())
}

/// Fetch active proposals for a group
#[wasm_bindgen(js_name = fetchGroupProposals)]
pub async fn fetch_group_proposals(
    sdk: &WasmSdk,
    group_id: &str,
    active_only: bool,
) -> Result<Array, JsError> {
    let _sdk = sdk;
    let _identifier = Identifier::from_string(
        group_id,
        platform_value::string_encoding::Encoding::Base58,
    )
    .map_err(|e| JsError::new(&format!("Invalid group ID: {}", e)))?;

    // Fetch proposals from platform
    let config = DapiClientConfig::new(sdk.network());
    let client = DapiClient::new(config)?;
    
    // Query for proposal documents
    let query = Object::new();
    let where_clause = js_sys::Array::new();
    let group_condition = js_sys::Array::of3(
        &"groupId".into(),
        &"==".into(),
        &group_id.into()
    );
    where_clause.push(&group_condition);
    
    if active_only {
        // Add condition for non-executed proposals
        let executed_condition = js_sys::Array::of3(
            &"executed".into(),
            &"==".into(),
            &false.into()
        );
        where_clause.push(&executed_condition);
        
        // Add condition for non-expired proposals
        let expires_condition = js_sys::Array::of3(
            &"expiresAt".into(),
            &">".into(),
            &(Date::now() as u64).into()
        );
        where_clause.push(&expires_condition);
    }
    
    Reflect::set(&query, &"where".into(), &where_clause)
        .map_err(|_| JsError::new("Failed to set where clause"))?;
    Reflect::set(&query, &"limit".into(), &100.into())
        .map_err(|_| JsError::new("Failed to set limit"))?;
    
    // Order by creation date descending
    let order_by = js_sys::Array::of2(
        &js_sys::Array::of2(&"createdAt".into(), &"desc".into()),
        &js_sys::Array::of2(&"$id".into(), &"asc".into())
    );
    Reflect::set(&query, &"orderBy".into(), &order_by)
        .map_err(|_| JsError::new("Failed to set orderBy"))?;
    
    let groups_contract_id = "GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec"; // System groups contract
    let documents = client.get_documents(
        groups_contract_id.to_string(),
        "groupProposal".to_string(),
        query.into(),
        JsValue::null(),
        100,
        None,
        false
    ).await?;
    
    // Parse and return the proposals array
    if let Some(docs_array) = js_sys::Reflect::get(&documents, &"documents".into())
        .map_err(|_| JsError::new("Failed to get documents from response"))?
        .dyn_ref::<js_sys::Array>() {
        return Ok(docs_array.clone());
    }
    
    Ok(Array::new())
}

/// Fetch user's groups
#[wasm_bindgen(js_name = fetchUserGroups)]
pub async fn fetch_user_groups(
    sdk: &WasmSdk,
    user_id: &str,
) -> Result<Array, JsError> {
    let _sdk = sdk;
    let _identifier = Identifier::from_string(
        user_id,
        platform_value::string_encoding::Encoding::Base58,
    )
    .map_err(|e| JsError::new(&format!("Invalid user ID: {}", e)))?;

    // Fetch user's groups from platform
    let config = DapiClientConfig::new(sdk.network());
    let client = DapiClient::new(config)?;
    
    // Query for groups where user is a member
    let query = Object::new();
    let where_clause = js_sys::Array::new();
    let member_condition = js_sys::Array::of3(
        &"members".into(),
        &"contains".into(),
        &user_id.into()
    );
    where_clause.push(&member_condition);
    
    Reflect::set(&query, &"where".into(), &where_clause)
        .map_err(|_| JsError::new("Failed to set where clause"))?;
    Reflect::set(&query, &"limit".into(), &100.into())
        .map_err(|_| JsError::new("Failed to set limit"))?;
    
    let groups_contract_id = "GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec"; // System groups contract
    let documents = client.get_documents(
        groups_contract_id.to_string(),
        "group".to_string(),
        query.into(),
        JsValue::null(),
        100,
        None,
        false
    ).await?;
    
    // Parse and return the groups array
    if let Some(docs_array) = js_sys::Reflect::get(&documents, &"documents".into())
        .map_err(|_| JsError::new("Failed to get documents from response"))?
        .dyn_ref::<js_sys::Array>() {
        return Ok(docs_array.clone());
    }
    
    Ok(Array::new())
}

/// Check if user can perform action in group
#[wasm_bindgen(js_name = checkGroupPermission)]
pub async fn check_group_permission(
    sdk: &WasmSdk,
    group_id: &str,
    user_id: &str,
    permission: &str,
) -> Result<bool, JsError> {
    let _sdk = sdk;
    let _group = Identifier::from_string(
        group_id,
        platform_value::string_encoding::Encoding::Base58,
    )
    .map_err(|e| JsError::new(&format!("Invalid group ID: {}", e)))?;

    let _user = Identifier::from_string(
        user_id,
        platform_value::string_encoding::Encoding::Base58,
    )
    .map_err(|e| JsError::new(&format!("Invalid user ID: {}", e)))?;

    // Fetch user's membership in the group to check permissions
    let config = DapiClientConfig::new(sdk.network());
    let client = DapiClient::new(config)?;
    
    // Query for member document
    let query = Object::new();
    let where_clause = js_sys::Array::new();
    
    // Group ID condition
    let group_condition = js_sys::Array::of3(
        &"groupId".into(),
        &"==".into(),
        &group_id.into()
    );
    where_clause.push(&group_condition);
    
    // User ID condition
    let user_condition = js_sys::Array::of3(
        &"identityId".into(),
        &"==".into(),
        &user_id.into()
    );
    where_clause.push(&user_condition);
    
    Reflect::set(&query, &"where".into(), &where_clause)
        .map_err(|_| JsError::new("Failed to set where clause"))?;
    Reflect::set(&query, &"limit".into(), &1.into())
        .map_err(|_| JsError::new("Failed to set limit"))?;
    
    let groups_contract_id = "GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec"; // System groups contract
    let documents = client.get_documents(
        groups_contract_id.to_string(),
        "groupMember".to_string(),
        query.into(),
        JsValue::null(),
        1,
        None,
        false
    ).await?;
    
    // Check if member exists and has permission
    if let Some(docs_array) = js_sys::Reflect::get(&documents, &"documents".into())
        .map_err(|_| JsError::new("Failed to get documents from response"))?
        .dyn_ref::<js_sys::Array>() {
        if docs_array.length() > 0 {
            let member_doc = docs_array.get(0);
            
            // Check permissions array
            if let Some(permissions_array) = js_sys::Reflect::get(&member_doc, &"permissions".into())
                .map_err(|_| JsError::new("Failed to get permissions from member"))?
                .dyn_ref::<js_sys::Array>() {
                // Check if user has the specific permission or "all" permission
                for i in 0..permissions_array.length() {
                    if let Some(perm) = permissions_array.get(i).as_string() {
                        if perm == permission || perm == "all" {
                            return Ok(true);
                        }
                    }
                }
            }
            
            // Check role-based permissions
            if let Some(role) = js_sys::Reflect::get(&member_doc, &"role".into())
                .map_err(|_| JsError::new("Failed to get role from member"))?
                .as_string() {
                match (role.as_str(), permission) {
                    ("owner", _) => return Ok(true), // Owners have all permissions
                    ("admin", perm) if perm != "delete_group" => return Ok(true), // Admins have most permissions
                    ("member", perm) if perm == "read" || perm == "propose" => return Ok(true), // Members can read and propose
                    ("observer", "read") => return Ok(true), // Observers can only read
                    _ => {}
                }
            }
        }
    }
    
    Ok(false)
}