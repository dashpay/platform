/* tslint:disable */
/* eslint-disable */
/**
* @param {any} state_repository
* @param {DocumentsBatchTransition} transition
* @returns {Promise<void>}
*/
export function applyDocumentsBatchTransition(state_repository: any, transition: DocumentsBatchTransition): Promise<void>;
/**
* @param {any} state_repository
* @param {DocumentsBatchTransition} state_transition
* @returns {Promise<ValidationResult>}
*/
export function validateDocumentsBatchTransitionState(state_repository: any, state_transition: DocumentsBatchTransition): Promise<ValidationResult>;
/**
* @param {any} state_repository
* @param {IdentityCreateTransition} state_transition
* @returns {Promise<void>}
*/
export function applyIdentityCreateTransition(state_repository: any, state_transition: IdentityCreateTransition): Promise<void>;
/**
* @param {any} state_repository
* @param {IdentityTopUpTransition} state_transition
* @returns {Promise<void>}
*/
export function applyIdentityTopUpTransition(state_repository: any, state_transition: IdentityTopUpTransition): Promise<void>;
/**
* @param {any} state_repository
* @param {IdentityUpdateTransition} state_transition
* @returns {Promise<void>}
*/
export function applyIdentityUpdateTransition(state_repository: any, state_transition: IdentityUpdateTransition): Promise<void>;
/**
* @param {Array<any>} operations
* @param {any} identity_id
* @returns {FeeResult}
*/
export function calculateStateTransitionFeeFromOperations(operations: Array<any>, identity_id: any): FeeResult;
/**
* @param {any} state_repository
* @param {DataContractCreateTransition} state_transition
* @returns {Promise<ValidationResult>}
*/
export function validateDataContractCreateTransitionState(state_repository: any, state_transition: DataContractCreateTransition): Promise<ValidationResult>;
/**
* @param {any} raw_parameters
* @returns {Promise<ValidationResult>}
*/
export function validateDataContractCreateTransitionBasic(raw_parameters: any): Promise<ValidationResult>;
/**
* @param {Array<any>} js_raw_transitions
* @param {DataContract} data_contract
* @param {any} owner_id
* @returns {any[]}
*/
export function findDuplicatesByIndices(js_raw_transitions: Array<any>, data_contract: DataContract, owner_id: any): any[];
/**
* @param {any} js_data_contract_id
* @param {string} document_type
* @param {string} transition_action_string
* @param {Array<any>} data_triggers_list
* @returns {Array<any>}
*/
export function getDataTriggers(js_data_contract_id: any, document_type: string, transition_action_string: string, data_triggers_list: Array<any>): Array<any>;
/**
* @returns {Array<any>}
*/
export function getAllDataTriggers(): Array<any>;
/**
* @param {any} state_repository
* @param {any} js_owner_id
* @param {Array<any>} js_document_transitions
* @param {DataContract} js_data_contract
* @param {StateTransitionExecutionContext} js_execution_context
* @returns {Promise<ValidationResult>}
*/
export function validateDocumentsUniquenessByIndices(state_repository: any, js_owner_id: any, js_document_transitions: Array<any>, js_data_contract: DataContract, js_execution_context: StateTransitionExecutionContext): Promise<ValidationResult>;
/**
* @param {any} raw_parameters
* @returns {any}
*/
export function createAssetLockProofInstance(raw_parameters: any): any;
/**
* @param {any} state_repository
* @param {string} raw_transaction
* @param {number} output_index
* @param {StateTransitionExecutionContext} execution_context
* @returns {Promise<ValidationResult>}
*/
export function validateAssetLockTransaction(state_repository: any, raw_transaction: string, output_index: number, execution_context: StateTransitionExecutionContext): Promise<ValidationResult>;
/**
* @param {any} state_repository
* @param {any} raw_asset_lock_proof
* @param {StateTransitionExecutionContext} execution_context
* @returns {Promise<any>}
*/
export function fetchAssetLockTransactionOutput(state_repository: any, raw_asset_lock_proof: any, execution_context: StateTransitionExecutionContext): Promise<any>;
/**
* @param {any} state_repository
* @param {any} raw_asset_lock_proof
* @param {StateTransitionExecutionContext} execution_context
* @returns {Promise<any>}
*/
export function fetchAssetLockPublicKeyHash(state_repository: any, raw_asset_lock_proof: any, execution_context: StateTransitionExecutionContext): Promise<any>;
/**
* @returns {any}
*/
export function generateTemporaryEcdsaPrivateKey(): any;
/**
* @param {Array<any>} js_document_transitions
* @param {DataTriggerExecutionContext} js_context
* @param {Array<any>} js_data_triggers
* @returns {Promise<Array<any>>}
*/
export function executeDataTriggers(js_document_transitions: Array<any>, js_context: DataTriggerExecutionContext, js_data_triggers: Array<any>): Promise<Array<any>>;
/**
* @param {any} contract_id
* @param {any} owner_id
* @param {string} document_type
* @param {Uint8Array} entropy
* @returns {any}
*/
export function generateDocumentId(contract_id: any, owner_id: any, document_type: string, entropy: Uint8Array): any;
/**
* @param {Uint8Array} buffer
* @returns {Array<any>}
*/
export function decodeProtocolEntity(buffer: Uint8Array): Array<any>;
/**
* @param {any} state_repository
* @param {Array<any>} js_document_transitions
* @param {StateTransitionExecutionContext} js_execution_context
* @returns {Promise<Array<any>>}
*/
export function fetchExtendedDocuments(state_repository: any, js_document_transitions: Array<any>, js_execution_context: StateTransitionExecutionContext): Promise<Array<any>>;
/**
* @param {any} external_state_repository
* @param {any} js_state_transition
* @param {any} bls_adapter
* @returns {Promise<ValidationResult>}
*/
export function validateStateTransitionIdentitySignature(external_state_repository: any, js_state_transition: any, bls_adapter: any): Promise<ValidationResult>;
/**
* @param {any} state_repository
* @param {any} js_raw_document
* @returns {Promise<ValidationResult>}
*/
export function fetchAndValidateDataContract(state_repository: any, js_raw_document: any): Promise<ValidationResult>;
/**
* @param {Array<any>} js_raw_transitions
* @returns {any[]}
*/
export function findDuplicatesById(js_raw_transitions: Array<any>): any[];
/**
* @param {Array<any>} js_raw_transitions
* @param {DataContract} data_contract
* @returns {ValidationResult}
*/
export function validatePartialCompoundIndices(js_raw_transitions: Array<any>, data_contract: DataContract): ValidationResult;
/**
* @param {ProtocolVersionValidator} protocol_version_validator
* @param {any} state_repository
* @param {any} js_raw_state_transition
* @param {StateTransitionExecutionContext} execution_context
* @returns {Promise<ValidationResult>}
*/
export function validateDocumentsBatchTransitionBasic(protocol_version_validator: ProtocolVersionValidator, state_repository: any, js_raw_state_transition: any, execution_context: StateTransitionExecutionContext): Promise<ValidationResult>;
/**
* @param {Uint8Array} bytes
* @returns {any}
*/
export function deserializeConsensusError(bytes: Uint8Array): any;
/**
* @param {any} state_repository
* @param {DataContractUpdateTransition} state_transition
* @returns {Promise<ValidationResult>}
*/
export function validateDataContractUpdateTransitionState(state_repository: any, state_transition: DataContractUpdateTransition): Promise<ValidationResult>;
/**
* @param {any} old_documents_schema
* @param {any} new_documents_schema
* @returns {ValidationResult}
*/
export function validateIndicesAreBackwardCompatible(old_documents_schema: any, new_documents_schema: any): ValidationResult;
/**
* @param {any} state_repository
* @param {any} raw_parameters
* @param {StateTransitionExecutionContext} execution_context
* @returns {Promise<ValidationResult>}
*/
export function validateDataContractUpdateTransitionBasic(state_repository: any, raw_parameters: any, execution_context: StateTransitionExecutionContext): Promise<ValidationResult>;
/**
* @param {any} state_transition_js
* @returns {FeeResult}
*/
export function calculateStateTransitionFee(state_transition_js: any): FeeResult;
/**
* @param {Array<any>} operations
* @returns {DummyFeesResult}
*/
export function calculateOperationFees(operations: Array<any>): DummyFeesResult;
/**
*/
export enum KeyType {
  ECDSA_SECP256K1 = 0,
  BLS12_381 = 1,
  ECDSA_HASH160 = 2,
  BIP13_SCRIPT_HASH = 3,
}
/**
*/
export enum KeySecurityLevel {
  MASTER = 0,
  CRITICAL = 1,
  HIGH = 2,
  MEDIUM = 3,
}
/**
*/
export enum KeyPurpose {
/**
* at least one authentication key must be registered for all security levels
*/
  AUTHENTICATION = 0,
/**
* this key cannot be used for signing documents
*/
  ENCRYPTION = 1,
/**
* this key cannot be used for signing documents
*/
  DECRYPTION = 2,
  WITHDRAW = 3,
}
/**
*/
export class ApplyDataContractCreateTransition {
  free(): void;
/**
* @param {any} state_repository
*/
  constructor(state_repository: any);
/**
* @param {DataContractCreateTransition} transition
* @returns {Promise<void>}
*/
  applyDataContractCreateTransition(transition: DataContractCreateTransition): Promise<void>;
}
/**
*/
export class ApplyDataContractUpdateTransition {
  free(): void;
/**
* @param {any} state_repository
*/
  constructor(state_repository: any);
/**
* @param {DataContractUpdateTransition} transition
* @returns {Promise<void>}
*/
  applyDataContractUpdateTransition(transition: DataContractUpdateTransition): Promise<void>;
}
/**
*/
export class AssetLockOutputNotFoundError {
  free(): void;
}
/**
*/
export class AssetLockProof {
  free(): void;
/**
* @param {any} raw_asset_lock_proof
*/
  constructor(raw_asset_lock_proof: any);
/**
* @returns {any}
*/
  createIdentifier(): any;
/**
* @returns {any}
*/
  toObject(): any;
}
/**
*/
export class AssetLockTransactionIsNotFoundError {
  free(): void;
/**
* @returns {any}
*/
  getTransactionId(): any;
}
/**
*/
export class BalanceIsNotEnoughError {
  free(): void;
/**
* @param {bigint} balance
* @param {bigint} fee
*/
  constructor(balance: bigint, fee: bigint);
/**
* @returns {bigint}
*/
  getBalance(): bigint;
/**
* @returns {bigint}
*/
  getFee(): bigint;
/**
* @returns {number}
*/
  getCode(): number;
/**
* @returns {any}
*/
  serialize(): any;
/**
*/
  readonly message: string;
}
/**
*/
export class ChainAssetLockProof {
  free(): void;
/**
* @param {any} raw_parameters
*/
  constructor(raw_parameters: any);
/**
* @returns {number}
*/
  getType(): number;
/**
* @returns {number}
*/
  getCoreChainLockedHeight(): number;
/**
* @param {number} value
*/
  setCoreChainLockedHeight(value: number): void;
/**
* @returns {any}
*/
  getOutPoint(): any;
/**
* @param {Uint8Array} out_point
*/
  setOutPoint(out_point: Uint8Array): void;
/**
* @returns {any}
*/
  toJSON(): any;
/**
* @returns {any}
*/
  toObject(): any;
/**
* @returns {any}
*/
  createIdentifier(): any;
}
/**
*/
export class ChainAssetLockProofStructureValidator {
  free(): void;
/**
* @param {any} state_repository
*/
  constructor(state_repository: any);
/**
* @param {any} raw_asset_lock_proof
* @param {StateTransitionExecutionContext} execution_context
* @returns {Promise<ValidationResult>}
*/
  validate(raw_asset_lock_proof: any, execution_context: StateTransitionExecutionContext): Promise<ValidationResult>;
}
/**
*/
export class CompatibleProtocolVersionIsNotDefinedError {
  free(): void;
/**
* @returns {number}
*/
  getCurrentProtocolVersion(): number;
}
/**
*/
export class DashPlatformProtocol {
  free(): void;
/**
* @param {any} bls_adapter
* @param {any} state_repository
* @param {any} entropy_generator
* @param {number | undefined} maybe_protocol_version
*/
  constructor(bls_adapter: any, state_repository: any, entropy_generator: any, maybe_protocol_version?: number);
/**
* @returns {number}
*/
  getProtocolVersion(): number;
/**
* @param {number} protocol_version
*/
  setProtocolVersion(protocol_version: number): void;
/**
* @param {any} state_repository
*/
  setStateRepository(state_repository: any): void;
/**
* @returns {any}
*/
  getStateRepository(): any;
/**
*/
  readonly dataContract: DataContractFacade;
/**
*/
  readonly document: DocumentFacade;
/**
*/
  readonly identity: IdentityFacade;
/**
*/
  readonly protocolVersion: number;
/**
*/
  readonly stateTransition: StateTransitionFacade;
}
/**
*/
export class DataContract {
  free(): void;
/**
* @param {any} raw_parameters
*/
  constructor(raw_parameters: any);
/**
* @returns {number}
*/
  getProtocolVersion(): number;
/**
* @returns {any}
*/
  getId(): any;
/**
* @param {any} id
*/
  setId(id: any): void;
/**
* @returns {any}
*/
  getOwnerId(): any;
/**
* @returns {number}
*/
  getVersion(): number;
/**
* @param {number} v
*/
  setVersion(v: number): void;
/**
*/
  incrementVersion(): void;
/**
* @returns {string}
*/
  getJsonSchemaId(): string;
/**
* @param {string} schema
*/
  setJsonMetaSchema(schema: string): void;
/**
* @returns {string}
*/
  getJsonMetaSchema(): string;
/**
* @param {any} documents
*/
  setDocuments(documents: any): void;
/**
* @returns {any}
*/
  getDocuments(): any;
/**
* @param {string} doc_type
* @returns {boolean}
*/
  isDocumentDefined(doc_type: string): boolean;
/**
* @param {string} doc_type
* @param {any} schema
*/
  setDocumentSchema(doc_type: string, schema: any): void;
/**
* @param {string} doc_type
* @returns {any}
*/
  getDocumentSchema(doc_type: string): any;
/**
* @param {string} doc_type
* @returns {any}
*/
  getDocumentSchemaRef(doc_type: string): any;
/**
* @param {any} definitions
*/
  setDefinitions(definitions: any): void;
/**
* @returns {any}
*/
  getDefinitions(): any;
/**
* @param {Uint8Array} e
*/
  setEntropy(e: Uint8Array): void;
/**
* @returns {any}
*/
  getEntropy(): any;
/**
* @param {string} doc_type
* @returns {any}
*/
  getBinaryProperties(doc_type: string): any;
/**
* @returns {Metadata | undefined}
*/
  getMetadata(): Metadata | undefined;
/**
* @param {any} metadata
*/
  setMetadata(metadata: any): void;
/**
* @returns {any}
*/
  toObject(): any;
/**
* @returns {any}
*/
  toJSON(): any;
/**
* @returns {any}
*/
  toBuffer(): any;
/**
* @returns {Uint8Array}
*/
  hash(): Uint8Array;
/**
* @param {any} v
* @returns {DataContract}
*/
  static from(v: any): DataContract;
/**
* @param {Uint8Array} b
* @returns {DataContract}
*/
  static fromBuffer(b: Uint8Array): DataContract;
/**
* @returns {DataContract}
*/
  clone(): DataContract;
}
/**
*/
export class DataContractAlreadyPresentError {
  free(): void;
/**
* @param {any} data_contract_id
*/
  constructor(data_contract_id: any);
/**
* @returns {any}
*/
  getDataContractId(): any;
/**
* @returns {number}
*/
  getCode(): number;
/**
* @returns {any}
*/
  serialize(): any;
/**
*/
  readonly message: string;
}
/**
*/
export class DataContractCreateTransition {
  free(): void;
/**
* @param {any} raw_parameters
*/
  constructor(raw_parameters: any);
/**
* @returns {DataContract}
*/
  getDataContract(): DataContract;
/**
* @returns {number}
*/
  getProtocolVersion(): number;
/**
* @returns {any}
*/
  getEntropy(): any;
/**
* @returns {any}
*/
  getOwnerId(): any;
/**
* @returns {number}
*/
  getType(): number;
/**
* @param {boolean | undefined} skip_signature
* @returns {any}
*/
  toJSON(skip_signature?: boolean): any;
/**
* @param {boolean | undefined} skip_signature
* @returns {any}
*/
  toBuffer(skip_signature?: boolean): any;
/**
* @returns {any[]}
*/
  getModifiedDataIds(): any[];
/**
* @returns {boolean}
*/
  isDataContractStateTransition(): boolean;
/**
* @returns {boolean}
*/
  isDocumentStateTransition(): boolean;
/**
* @returns {boolean}
*/
  isIdentityStateTransition(): boolean;
/**
* @param {StateTransitionExecutionContext} context
*/
  setExecutionContext(context: StateTransitionExecutionContext): void;
/**
* @returns {StateTransitionExecutionContext}
*/
  getExecutionContext(): StateTransitionExecutionContext;
/**
* @param {boolean | undefined} skip_signature
* @returns {any}
*/
  toObject(skip_signature?: boolean): any;
/**
* @param {IdentityPublicKey} identity_public_key
* @param {Uint8Array} private_key
* @param {any} bls
*/
  sign(identity_public_key: IdentityPublicKey, private_key: Uint8Array, bls: any): void;
/**
* @param {IdentityPublicKey} identity_public_key
* @param {any} bls
* @returns {boolean}
*/
  verifySignature(identity_public_key: IdentityPublicKey, bls: any): boolean;
}
/**
*/
export class DataContractDefaults {
  free(): void;
/**
*/
  static readonly SCHEMA: string;
}
/**
*/
export class DataContractFacade {
  free(): void;
/**
* Create Data Contract
* @param {Uint8Array} owner_id
* @param {any} documents
* @param {any} definitions
* @returns {DataContract}
*/
  create(owner_id: Uint8Array, documents: any, definitions: any): DataContract;
/**
* Create Data Contract from plain object
* @param {any} js_raw_data_contract
* @param {any} options
* @returns {Promise<DataContract>}
*/
  createFromObject(js_raw_data_contract: any, options: any): Promise<DataContract>;
/**
* Create Data Contract from buffer
* @param {Uint8Array} buffer
* @param {any} options
* @returns {Promise<DataContract>}
*/
  createFromBuffer(buffer: Uint8Array, options: any): Promise<DataContract>;
/**
* Create Data Contract Create State Transition
* @param {DataContract} data_contract
* @returns {DataContractCreateTransition}
*/
  createDataContractCreateTransition(data_contract: DataContract): DataContractCreateTransition;
/**
* Create Data Contract Update State Transition
* @param {DataContract} data_contract
* @returns {DataContractUpdateTransition}
*/
  createDataContractUpdateTransition(data_contract: DataContract): DataContractUpdateTransition;
/**
* Validate Data Contract
* @param {any} js_raw_data_contract
* @returns {Promise<ValidationResult>}
*/
  validate(js_raw_data_contract: any): Promise<ValidationResult>;
}
/**
*/
export class DataContractFactory {
  free(): void;
/**
* @param {number} protocol_version
* @param {DataContractValidator} validate_data_contract
* @param {any | undefined} external_entropy_generator_arg
*/
  constructor(protocol_version: number, validate_data_contract: DataContractValidator, external_entropy_generator_arg?: any);
/**
* @param {Uint8Array} owner_id
* @param {any} documents
* @returns {DataContract}
*/
  create(owner_id: Uint8Array, documents: any): DataContract;
/**
* @param {any} object
* @param {boolean | undefined} skip_validation
* @returns {Promise<DataContract>}
*/
  createFromObject(object: any, skip_validation?: boolean): Promise<DataContract>;
/**
* @param {Uint8Array} buffer
* @param {boolean | undefined} skip_validation
* @returns {Promise<DataContract>}
*/
  createFromBuffer(buffer: Uint8Array, skip_validation?: boolean): Promise<DataContract>;
/**
* @param {DataContract} data_contract
* @returns {Promise<DataContractCreateTransition>}
*/
  createDataContractCreateTransition(data_contract: DataContract): Promise<DataContractCreateTransition>;
}
/**
*/
export class DataContractGenericError {
  free(): void;
/**
* @returns {string}
*/
  getMessage(): string;
}
/**
*/
export class DataContractHaveNewUniqueIndexError {
  free(): void;
/**
* @returns {string}
*/
  getDocumentType(): string;
/**
* @returns {string}
*/
  getIndexName(): string;
/**
* @returns {number}
*/
  getCode(): number;
/**
* @returns {any}
*/
  serialize(): any;
/**
*/
  readonly message: string;
}
/**
*/
export class DataContractImmutablePropertiesUpdateError {
  free(): void;
/**
* @returns {string}
*/
  getOperation(): string;
/**
* @returns {string}
*/
  getFieldPath(): string;
/**
* @returns {number}
*/
  getCode(): number;
/**
* @returns {any}
*/
  serialize(): any;
/**
*/
  readonly message: string;
}
/**
*/
export class DataContractInvalidIndexDefinitionUpdateError {
  free(): void;
/**
* @returns {string}
*/
  getDocumentType(): string;
/**
* @returns {string}
*/
  getIndexName(): string;
/**
* @returns {number}
*/
  getCode(): number;
/**
* @returns {any}
*/
  serialize(): any;
/**
*/
  readonly message: string;
}
/**
*/
export class DataContractMaxDepthExceedError {
  free(): void;
}
/**
*/
export class DataContractNotPresentError {
  free(): void;
/**
* @returns {any}
*/
  getDataContractId(): any;
/**
* @returns {number}
*/
  getCode(): number;
/**
* @returns {any}
*/
  serialize(): any;
/**
*/
  readonly message: string;
}
/**
*/
export class DataContractNotPresentNotConsensusError {
  free(): void;
/**
* @returns {any}
*/
  getDataContractId(): any;
}
/**
*/
export class DataContractUniqueIndicesChangedError {
  free(): void;
/**
* @returns {string}
*/
  getDocumentType(): string;
/**
* @returns {string}
*/
  getIndexName(): string;
/**
* @returns {number}
*/
  getCode(): number;
/**
* @returns {any}
*/
  serialize(): any;
/**
*/
  readonly message: string;
}
/**
*/
export class DataContractUpdateTransition {
  free(): void;
/**
* @param {any} raw_parameters
*/
  constructor(raw_parameters: any);
/**
* @returns {DataContract}
*/
  getDataContract(): DataContract;
/**
* @returns {number}
*/
  getProtocolVersion(): number;
/**
* @returns {any}
*/
  getEntropy(): any;
/**
* @returns {any}
*/
  getOwnerId(): any;
/**
* @returns {number}
*/
  getType(): number;
/**
* @param {boolean | undefined} skip_signature
* @returns {any}
*/
  toJSON(skip_signature?: boolean): any;
/**
* @param {boolean | undefined} skip_signature
* @returns {any}
*/
  toBuffer(skip_signature?: boolean): any;
/**
* @returns {any[]}
*/
  getModifiedDataIds(): any[];
/**
* @returns {boolean}
*/
  isDataContractStateTransition(): boolean;
/**
* @returns {boolean}
*/
  isDocumentStateTransition(): boolean;
/**
* @returns {boolean}
*/
  isIdentityStateTransition(): boolean;
/**
* @param {StateTransitionExecutionContext} context
*/
  setExecutionContext(context: StateTransitionExecutionContext): void;
/**
* @returns {StateTransitionExecutionContext}
*/
  getExecutionContext(): StateTransitionExecutionContext;
/**
* @param {boolean | undefined} skip_signature
* @returns {any}
*/
  hash(skip_signature?: boolean): any;
/**
* @param {boolean | undefined} skip_signature
* @returns {any}
*/
  toObject(skip_signature?: boolean): any;
/**
* @param {IdentityPublicKey} identity_public_key
* @param {Uint8Array} private_key
* @param {any} bls
*/
  sign(identity_public_key: IdentityPublicKey, private_key: Uint8Array, bls: any): void;
/**
* @param {IdentityPublicKey} identity_public_key
* @param {any} bls
* @returns {boolean}
*/
  verifySignature(identity_public_key: IdentityPublicKey, bls: any): boolean;
}
/**
*/
export class DataContractValidator {
  free(): void;
/**
*/
  constructor();
/**
* @param {any} raw_data_contract
* @returns {ValidationResult}
*/
  validate(raw_data_contract: any): ValidationResult;
}
/**
*/
export class DataTrigger {
  free(): void;
/**
*/
  dataContractId: any;
/**
*/
  readonly dataTriggerKind: string;
/**
*/
  documentType: string;
/**
*/
  topLevelIdentity: any;
/**
*/
  transitionAction: string;
}
/**
*/
export class DataTriggerConditionError {
  free(): void;
/**
* @returns {any}
*/
  getDataContractId(): any;
/**
* @returns {any}
*/
  getDocumentId(): any;
/**
* @returns {string}
*/
  getMessage(): string;
/**
* @returns {number}
*/
  getCode(): number;
/**
* @returns {any}
*/
  serialize(): any;
/**
*/
  readonly message: string;
}
/**
*/
export class DataTriggerExecutionContext {
  free(): void;
/**
* @param {any} state_repository
* @param {any} js_owner_id
* @param {DataContract} data_contract
* @param {StateTransitionExecutionContext} state_transition_execution_context
*/
  constructor(state_repository: any, js_owner_id: any, data_contract: DataContract, state_transition_execution_context: StateTransitionExecutionContext);
/**
*/
  dataContract: DataContract;
/**
*/
  ownerId: any;
/**
*/
  statTransitionExecutionContext: StateTransitionExecutionContext;
/**
*/
  readonly stateTransitionExecutionContext: StateTransitionExecutionContext;
}
/**
*/
export class DataTriggerExecutionError {
  free(): void;
/**
* @returns {any}
*/
  getDataContractId(): any;
/**
* @returns {any}
*/
  getDocumentId(): any;
/**
* @returns {string}
*/
  getMessage(): string;
/**
* @returns {number}
*/
  getCode(): number;
/**
* @returns {any}
*/
  serialize(): any;
/**
*/
  readonly message: string;
}
/**
*/
export class DataTriggerExecutionResult {
  free(): void;
/**
* @returns {boolean}
*/
  isOk(): boolean;
/**
* @returns {Array<any>}
*/
  getErrors(): Array<any>;
}
/**
*/
export class DataTriggerInvalidResultError {
  free(): void;
/**
* @returns {any}
*/
  getDataContractId(): any;
/**
* @returns {any}
*/
  getDocumentId(): any;
/**
* @returns {number}
*/
  getCode(): number;
/**
* @returns {any}
*/
  serialize(): any;
/**
*/
  readonly message: string;
}
/**
*/
export class Document {
  free(): void;
/**
* @param {any} js_raw_document
* @param {DataContract} js_data_contract
* @param {any} js_document_type_name
*/
  constructor(js_raw_document: any, js_data_contract: DataContract, js_document_type_name: any);
/**
* @returns {any}
*/
  getId(): any;
/**
* @param {any} js_id
*/
  setId(js_id: any): void;
/**
* @param {any} owner_id
*/
  setOwnerId(owner_id: any): void;
/**
* @returns {any}
*/
  getOwnerId(): any;
/**
* @param {number | undefined} revision
*/
  setRevision(revision?: number): void;
/**
* @returns {number | undefined}
*/
  getRevision(): number | undefined;
/**
* @param {any} d
*/
  setData(d: any): void;
/**
* @returns {any}
*/
  getData(): any;
/**
* @param {string} path
* @param {any} js_value_to_set
*/
  set(path: string, js_value_to_set: any): void;
/**
* @param {string} path
* @returns {any}
*/
  get(path: string): any;
/**
* @param {Date | undefined} created_at
*/
  setCreatedAt(created_at?: Date): void;
/**
* @param {Date | undefined} updated_at
*/
  setUpdatedAt(updated_at?: Date): void;
/**
* @returns {Date | undefined}
*/
  getCreatedAt(): Date | undefined;
/**
* @returns {Date | undefined}
*/
  getUpdatedAt(): Date | undefined;
/**
* @param {any} options
* @param {DataContract} data_contract
* @param {string} document_type_name
* @returns {any}
*/
  toObject(options: any, data_contract: DataContract, document_type_name: string): any;
/**
* @returns {any}
*/
  toJSON(): any;
/**
* @returns {any}
*/
  toBuffer(): any;
/**
* @param {DataContract} data_contract
* @param {string} document_type_name
* @returns {any}
*/
  hash(data_contract: DataContract, document_type_name: string): any;
/**
* @returns {Document}
*/
  clone(): Document;
}
/**
*/
export class DocumentAlreadyExistsError {
  free(): void;
/**
* @returns {any}
*/
  getDocumentTransition(): any;
}
/**
*/
export class DocumentAlreadyPresentError {
  free(): void;
/**
* @returns {any}
*/
  getDocumentId(): any;
/**
* @returns {number}
*/
  getCode(): number;
/**
* @returns {any}
*/
  serialize(): any;
/**
*/
  readonly message: string;
}
/**
*/
export class DocumentCreateTransition {
  free(): void;
/**
* @param {any} raw_object
* @param {DataContract} data_contract
*/
  constructor(raw_object: any, data_contract: DataContract);
/**
* @returns {Uint8Array}
*/
  getEntropy(): Uint8Array;
/**
* @returns {Date | undefined}
*/
  getCreatedAt(): Date | undefined;
/**
* @returns {Date | undefined}
*/
  getUpdatedAt(): Date | undefined;
/**
* @returns {bigint}
*/
  getRevision(): bigint;
/**
* @returns {any}
*/
  getId(): any;
/**
* @returns {string}
*/
  getType(): string;
/**
* @returns {number}
*/
  getAction(): number;
/**
* @returns {DataContract}
*/
  getDataContract(): DataContract;
/**
* @returns {any}
*/
  getDataContractId(): any;
/**
* @param {string} path
* @returns {any}
*/
  get(path: string): any;
/**
* @param {any} options
* @returns {any}
*/
  toObject(options: any): any;
/**
* @returns {any}
*/
  toJSON(): any;
/**
* @returns {any}
*/
  getData(): any;
}
/**
*/
export class DocumentDeleteTransition {
  free(): void;
/**
* @returns {number}
*/
  getAction(): number;
/**
* @param {any} options
* @returns {any}
*/
  toObject(options: any): any;
/**
* @returns {any}
*/
  toJSON(): any;
/**
* @returns {any}
*/
  getId(): any;
/**
* @returns {string}
*/
  getType(): string;
/**
* @returns {DataContract}
*/
  getDataContract(): DataContract;
/**
* @returns {any}
*/
  getDataContractId(): any;
/**
* @param {string} path
* @returns {any}
*/
  get(path: string): any;
}
/**
*/
export class DocumentFacade {
  free(): void;
/**
* @param {DocumentValidator} document_validator
* @param {DocumentFactory} document_factory
* @param {FetchAndValidateDataContractFactory} data_contract_fetcher_and_validator
*/
  constructor(document_validator: DocumentValidator, document_factory: DocumentFactory, data_contract_fetcher_and_validator: FetchAndValidateDataContractFactory);
/**
* @param {DataContract} data_contract
* @param {any} js_owner_id
* @param {string} document_type
* @param {any} data
* @returns {ExtendedDocument}
*/
  create(data_contract: DataContract, js_owner_id: any, document_type: string, data: any): ExtendedDocument;
/**
* Creates Document from object
* @param {any} raw_document
* @param {any} options
* @returns {Promise<ExtendedDocument>}
*/
  createFromObject(raw_document: any, options: any): Promise<ExtendedDocument>;
/**
* Creates Document form bytes
* @param {Uint8Array} bytes
* @param {any} options
* @returns {Promise<ExtendedDocument>}
*/
  createFromBuffer(bytes: Uint8Array, options: any): Promise<ExtendedDocument>;
/**
* Creates Documents State Transition
* @param {any} documents
* @returns {DocumentsBatchTransition}
*/
  createStateTransition(documents: any): DocumentsBatchTransition;
/**
* Creates Documents State Transition
* @param {any} document
* @returns {Promise<ValidationResult>}
*/
  validate(document: any): Promise<ValidationResult>;
/**
* Creates Documents State Transition
* @param {any} js_raw_document
* @returns {Promise<ValidationResult>}
*/
  validate_raw_document(js_raw_document: any): Promise<ValidationResult>;
}
/**
*/
export class DocumentFactory {
  free(): void;
/**
* @param {number} protocol_version
* @param {DocumentValidator} document_validator
* @param {any} state_repository
* @param {any | undefined} external_entropy_generator_arg
*/
  constructor(protocol_version: number, document_validator: DocumentValidator, state_repository: any, external_entropy_generator_arg?: any);
/**
* @param {DataContract} data_contract
* @param {any} js_owner_id
* @param {string} document_type
* @param {any} data
* @returns {ExtendedDocument}
*/
  create(data_contract: DataContract, js_owner_id: any, document_type: string, data: any): ExtendedDocument;
/**
* @param {any} documents
* @returns {DocumentsBatchTransition}
*/
  createStateTransition(documents: any): DocumentsBatchTransition;
/**
* @param {any} raw_document_js
* @param {any} options
* @returns {Promise<ExtendedDocument>}
*/
  createFromObject(raw_document_js: any, options: any): Promise<ExtendedDocument>;
/**
* @param {Uint8Array} buffer
* @param {any} options
* @returns {Promise<ExtendedDocument>}
*/
  createFromBuffer(buffer: Uint8Array, options: any): Promise<ExtendedDocument>;
}
/**
*/
export class DocumentNoRevisionError {
  free(): void;
/**
* @param {Document} document
*/
  constructor(document: Document);
/**
* @returns {Document}
*/
  getDocument(): Document;
}
/**
*/
export class DocumentNotFoundError {
  free(): void;
/**
* @returns {any}
*/
  getDocumentId(): any;
/**
* @returns {number}
*/
  getCode(): number;
/**
* @returns {any}
*/
  serialize(): any;
/**
*/
  readonly message: string;
}
/**
*/
export class DocumentNotProvidedError {
  free(): void;
/**
* @returns {any}
*/
  getDocumentTransition(): any;
}
/**
*/
export class DocumentOwnerIdMismatchError {
  free(): void;
/**
* @returns {any}
*/
  getDocumentId(): any;
/**
* @returns {any}
*/
  getDocumentOwnerId(): any;
/**
* @returns {any}
*/
  getExistingDocumentOwnerId(): any;
/**
* @returns {number}
*/
  getCode(): number;
/**
* @returns {any}
*/
  serialize(): any;
/**
*/
  readonly message: string;
}
/**
*/
export class DocumentReplaceTransition {
  free(): void;
/**
* @param {any} raw_object
* @param {DataContract} data_contract
*/
  constructor(raw_object: any, data_contract: DataContract);
/**
* @returns {number}
*/
  getAction(): number;
/**
* @returns {bigint}
*/
  getRevision(): bigint;
/**
* @returns {Date | undefined}
*/
  getUpdatedAt(): Date | undefined;
/**
* @param {any} options
* @returns {any}
*/
  toObject(options: any): any;
/**
* @returns {any}
*/
  toJSON(): any;
/**
* @returns {any}
*/
  getData(): any;
/**
* @returns {any}
*/
  getId(): any;
/**
* @returns {string}
*/
  getType(): string;
/**
* @returns {DataContract}
*/
  getDataContract(): DataContract;
/**
* @returns {any}
*/
  getDataContractId(): any;
/**
* @param {string} path
* @returns {any}
*/
  get(path: string): any;
}
/**
*/
export class DocumentTimestampWindowViolationError {
  free(): void;
/**
* @returns {any}
*/
  getDocumentId(): any;
/**
* @returns {string}
*/
  getTimestampName(): string;
/**
* @returns {Date}
*/
  getTimestamp(): Date;
/**
* @returns {Date}
*/
  getTimeWindowStart(): Date;
/**
* @returns {Date}
*/
  getTimeWindowEnd(): Date;
/**
* @returns {number}
*/
  getCode(): number;
/**
* @returns {any}
*/
  serialize(): any;
/**
*/
  readonly message: string;
}
/**
*/
export class DocumentTimestampsMismatchError {
  free(): void;
/**
* @returns {any}
*/
  getDocumentId(): any;
/**
* @returns {number}
*/
  getCode(): number;
/**
* @returns {any}
*/
  serialize(): any;
/**
*/
  readonly message: string;
}
/**
*/
export class DocumentTransition {
  free(): void;
/**
* @returns {any}
*/
  getId(): any;
/**
* @returns {string}
*/
  getType(): string;
/**
* @returns {number}
*/
  getAction(): number;
/**
* @returns {DataContract}
*/
  getDataContract(): DataContract;
/**
* @returns {any}
*/
  getDataContractId(): any;
/**
* @param {any} js_data_contract_id
*/
  setDataContractId(js_data_contract_id: any): void;
/**
* @returns {any}
*/
  getRevision(): any;
/**
* @returns {any}
*/
  getCreatedAt(): any;
/**
* @returns {any}
*/
  getUpdatedAt(): any;
/**
* @param {Date | undefined} updated_at
*/
  setUpdatedAt(updated_at?: Date): void;
/**
* @param {Date | undefined} created_at
*/
  setCreatedAt(created_at?: Date): void;
/**
* @returns {any}
*/
  getData(): any;
/**
* @param {string} path
* @returns {any}
*/
  get(path: string): any;
/**
* @param {any} options
* @returns {any}
*/
  toObject(options: any): any;
/**
* @returns {any}
*/
  toJSON(): any;
/**
* @param {DocumentCreateTransition} js_create_transition
* @returns {DocumentTransition}
*/
  static fromTransitionCreate(js_create_transition: DocumentCreateTransition): DocumentTransition;
/**
* @param {DocumentReplaceTransition} js_replace_transition
* @returns {DocumentTransition}
*/
  static fromTransitionReplace(js_replace_transition: DocumentReplaceTransition): DocumentTransition;
/**
* @param {DocumentDeleteTransition} js_delete_transition
* @returns {DocumentTransition}
*/
  static fromTransitionDelete(js_delete_transition: DocumentDeleteTransition): DocumentTransition;
}
/**
*/
export class DocumentTransitions {
  free(): void;
/**
*/
  constructor();
/**
* @param {ExtendedDocument} transition
*/
  addTransitionCreate(transition: ExtendedDocument): void;
/**
* @param {ExtendedDocument} transition
*/
  addTransitionReplace(transition: ExtendedDocument): void;
/**
* @param {ExtendedDocument} transition
*/
  addTransitionDelete(transition: ExtendedDocument): void;
}
/**
*/
export class DocumentValidator {
  free(): void;
/**
* @param {ProtocolVersionValidator} protocol_validator
*/
  constructor(protocol_validator: ProtocolVersionValidator);
/**
* @param {any} js_raw_document
* @param {DataContract} js_data_contract
* @returns {ValidationResult}
*/
  validate(js_raw_document: any, js_data_contract: DataContract): ValidationResult;
}
/**
*/
export class DocumentsBatchTransition {
  free(): void;
/**
* @param {any} js_raw_transition
* @param {Array<any>} data_contracts
*/
  constructor(js_raw_transition: any, data_contracts: Array<any>);
/**
* @returns {number}
*/
  getType(): number;
/**
* @returns {any}
*/
  getOwnerId(): any;
/**
* @returns {Array<any>}
*/
  getTransitions(): Array<any>;
/**
* @param {Array<any>} js_transitions
*/
  setTransitions(js_transitions: Array<any>): void;
/**
* @returns {any}
*/
  toJSON(): any;
/**
* @param {any} js_options
* @returns {any}
*/
  toObject(js_options: any): any;
/**
* @returns {Array<any>}
*/
  getModifiedDataIds(): Array<any>;
/**
* @returns {number | undefined}
*/
  getSignaturePublicKeyId(): number | undefined;
/**
* @param {IdentityPublicKey} identity_public_key
* @param {Uint8Array} private_key
* @param {any} bls
*/
  sign(identity_public_key: IdentityPublicKey, private_key: Uint8Array, bls: any): void;
/**
* @param {IdentityPublicKey} public_key
*/
  verifyPublicKeyLevelAndPurpose(public_key: IdentityPublicKey): void;
/**
* @param {IdentityPublicKey} public_key
*/
  verifyPublicKeyIsEnabled(public_key: IdentityPublicKey): void;
/**
* @param {IdentityPublicKey} identity_public_key
* @param {any} bls
* @returns {boolean}
*/
  verifySignature(identity_public_key: IdentityPublicKey, bls: any): boolean;
/**
* @param {number} key_id
*/
  setSignaturePublicKey(key_id: number): void;
/**
* @returns {number}
*/
  getKeySecurityLevelRequirement(): number;
/**
* @returns {number}
*/
  getProtocolVersion(): number;
/**
* @returns {Uint8Array}
*/
  getSignature(): Uint8Array;
/**
* @param {Uint8Array} signature
*/
  setSignature(signature: Uint8Array): void;
/**
* @returns {boolean}
*/
  isDocumentStateTransition(): boolean;
/**
* @returns {boolean}
*/
  isDataContractStateTransition(): boolean;
/**
* @returns {boolean}
*/
  isIdentityStateTransition(): boolean;
/**
* @param {StateTransitionExecutionContext} context
*/
  setExecutionContext(context: StateTransitionExecutionContext): void;
/**
* @returns {StateTransitionExecutionContext}
*/
  getExecutionContext(): StateTransitionExecutionContext;
/**
* @param {any} options
* @returns {any}
*/
  toBuffer(options: any): any;
/**
* @param {any} options
* @returns {any}
*/
  hash(options: any): any;
}
/**
*/
export class DummyFeesResult {
  free(): void;
/**
*/
  feeRefunds: Array<any>;
/**
*/
  processingFee: any;
/**
*/
  storageFee: any;
}
/**
*/
export class DuplicateDocumentTransitionsWithIdsError {
  free(): void;
/**
* @returns {Array<any>}
*/
  getDocumentTransitionReferences(): Array<any>;
/**
* @returns {number}
*/
  getCode(): number;
/**
* @returns {any}
*/
  serialize(): any;
/**
*/
  readonly message: string;
}
/**
*/
export class DuplicateDocumentTransitionsWithIndicesError {
  free(): void;
/**
* @returns {Array<any>}
*/
  getDocumentTransitionReferences(): Array<any>;
/**
* @returns {number}
*/
  getCode(): number;
/**
* @returns {any}
*/
  serialize(): any;
/**
*/
  readonly message: string;
}
/**
*/
export class DuplicateIndexError {
  free(): void;
/**
* @returns {string}
*/
  getDocumentType(): string;
/**
* @returns {string}
*/
  getIndexName(): string;
/**
* @returns {number}
*/
  getCode(): number;
/**
* @returns {any}
*/
  serialize(): any;
/**
*/
  readonly message: string;
}
/**
*/
export class DuplicateIndexNameError {
  free(): void;
/**
* @returns {string}
*/
  getDocumentType(): string;
/**
* @returns {string}
*/
  getDuplicateIndexName(): string;
/**
* @returns {number}
*/
  getCode(): number;
/**
* @returns {any}
*/
  serialize(): any;
/**
*/
  readonly message: string;
}
/**
*/
export class DuplicateUniqueIndexError {
  free(): void;
/**
* @returns {any}
*/
  getDocumentId(): any;
/**
* @returns {Array<any>}
*/
  getDuplicatingProperties(): Array<any>;
/**
* @returns {number}
*/
  getCode(): number;
/**
* @returns {any}
*/
  serialize(): any;
/**
*/
  readonly message: string;
}
/**
*/
export class DuplicatedIdentityPublicKeyError {
  free(): void;
/**
* @returns {Array<any>}
*/
  getDuplicatedPublicKeysIds(): Array<any>;
/**
* @returns {number}
*/
  getCode(): number;
/**
* @returns {any}
*/
  serialize(): any;
/**
*/
  readonly message: string;
}
/**
*/
export class DuplicatedIdentityPublicKeyIdError {
  free(): void;
/**
* @returns {Array<any>}
*/
  getDuplicatedIds(): Array<any>;
/**
* @returns {number}
*/
  getCode(): number;
/**
* @returns {any}
*/
  serialize(): any;
/**
*/
  readonly message: string;
}
/**
*/
export class DuplicatedIdentityPublicKeyIdStateError {
  free(): void;
/**
* @returns {Array<any>}
*/
  getDuplicatedIds(): Array<any>;
/**
* @returns {number}
*/
  getCode(): number;
/**
* @returns {any}
*/
  serialize(): any;
/**
*/
  readonly message: string;
}
/**
*/
export class DuplicatedIdentityPublicKeyStateError {
  free(): void;
/**
* @returns {Array<any>}
*/
  getDuplicatedPublicKeysIds(): Array<any>;
/**
* @returns {number}
*/
  getCode(): number;
/**
* @returns {any}
*/
  serialize(): any;
/**
*/
  readonly message: string;
}
/**
*/
export class ExtendedDocument {
  free(): void;
/**
* @param {any} js_raw_document
* @param {DataContract} js_data_contract
*/
  constructor(js_raw_document: any, js_data_contract: DataContract);
/**
* @returns {number}
*/
  getProtocolVersion(): number;
/**
* @returns {any}
*/
  getId(): any;
/**
* @param {any} js_id
*/
  setId(js_id: any): void;
/**
* @returns {string}
*/
  getType(): string;
/**
* @returns {any}
*/
  getDataContractId(): any;
/**
* @returns {DataContract}
*/
  getDataContract(): DataContract;
/**
* @param {any} js_data_contract_id
*/
  setDataContractId(js_data_contract_id: any): void;
/**
* @returns {Document}
*/
  getDocument(): Document;
/**
* @param {any} owner_id
*/
  setOwnerId(owner_id: any): void;
/**
* @returns {any}
*/
  getOwnerId(): any;
/**
* @param {number | undefined} rev
*/
  setRevision(rev?: number): void;
/**
* @returns {number | undefined}
*/
  getRevision(): number | undefined;
/**
* @param {Uint8Array} e
*/
  setEntropy(e: Uint8Array): void;
/**
* @returns {any}
*/
  getEntropy(): any;
/**
* @param {any} d
*/
  setData(d: any): void;
/**
* @returns {any}
*/
  getData(): any;
/**
* @param {string} path
* @param {any} js_value_to_set
*/
  set(path: string, js_value_to_set: any): void;
/**
* @param {string} path
* @returns {any}
*/
  get(path: string): any;
/**
* @param {Date | undefined} ts
*/
  setCreatedAt(ts?: Date): void;
/**
* @param {Date | undefined} ts
*/
  setUpdatedAt(ts?: Date): void;
/**
* @returns {Date | undefined}
*/
  getCreatedAt(): Date | undefined;
/**
* @returns {Date | undefined}
*/
  getUpdatedAt(): Date | undefined;
/**
* @returns {Metadata | undefined}
*/
  getMetadata(): Metadata | undefined;
/**
* @param {any} metadata
*/
  setMetadata(metadata: any): void;
/**
* @param {any} options
* @returns {any}
*/
  toObject(options: any): any;
/**
* @returns {any}
*/
  toJSON(): any;
/**
* @returns {any}
*/
  toBuffer(): any;
/**
* @returns {any}
*/
  hash(): any;
/**
* @returns {ExtendedDocument}
*/
  clone(): ExtendedDocument;
}
/**
*/
export class FeeResult {
  free(): void;
/**
*/
  constructor();
/**
*/
  desiredAmount: any;
/**
*/
  feeRefunds: Array<any>;
/**
*/
  processingFee: any;
/**
*/
  requiredAmount: any;
/**
*/
  storageFee: any;
/**
*/
  totalRefunds: any;
}
/**
*/
export class FetchAndValidateDataContractFactory {
  free(): void;
/**
* @param {any} state_repository
*/
  constructor(state_repository: any);
/**
* @param {any} js_raw_document
* @returns {Promise<ValidationResult>}
*/
  validate(js_raw_document: any): Promise<ValidationResult>;
}
/**
*/
export class Identity {
  free(): void;
/**
* @param {any} raw_identity
*/
  constructor(raw_identity: any);
/**
* @returns {number}
*/
  getProtocolVersion(): number;
/**
* @returns {any}
*/
  getId(): any;
/**
* @param {Array<any>} public_keys
* @returns {number}
*/
  setPublicKeys(public_keys: Array<any>): number;
/**
* @returns {any[]}
*/
  getPublicKeys(): any[];
/**
* @param {number} key_id
* @returns {IdentityPublicKey | undefined}
*/
  getPublicKeyById(key_id: number): IdentityPublicKey | undefined;
/**
* @returns {number}
*/
  getBalance(): number;
/**
* @param {number} balance
*/
  setBalance(balance: number): void;
/**
* @param {number} amount
* @returns {number}
*/
  increaseBalance(amount: number): number;
/**
* @param {number} amount
* @returns {number}
*/
  reduceBalance(amount: number): number;
/**
* @param {any} lock
*/
  setAssetLockProof(lock: any): void;
/**
* @returns {AssetLockProof | undefined}
*/
  getAssetLockProof(): AssetLockProof | undefined;
/**
* @param {number} revision
*/
  setRevision(revision: number): void;
/**
* @returns {number}
*/
  getRevision(): number;
/**
* @param {any} metadata
*/
  setMetadata(metadata: any): void;
/**
* @returns {Metadata | undefined}
*/
  getMetadata(): Metadata | undefined;
/**
* @param {any} object
* @returns {Identity}
*/
  static from(object: any): Identity;
/**
* @returns {any}
*/
  toJSON(): any;
/**
* @returns {any}
*/
  toObject(): any;
/**
* @returns {Uint8Array}
*/
  toBuffer(): Uint8Array;
/**
* @returns {Uint8Array}
*/
  hash(): Uint8Array;
/**
* @param {IdentityPublicKey} public_key
*/
  addPublicKey(public_key: IdentityPublicKey): void;
/**
* @param {...any} js_public_keys
*/
  addPublicKeys(...js_public_keys: any): void;
/**
* @returns {number}
*/
  getPublicKeyMaxId(): number;
}
/**
*/
export class IdentityAlreadyExistsError {
  free(): void;
/**
* @returns {any}
*/
  getIdentityId(): any;
/**
* @returns {number}
*/
  getCode(): number;
/**
* @returns {any}
*/
  serialize(): any;
/**
*/
  readonly message: string;
}
/**
*/
export class IdentityAssetLockProofLockedTransactionMismatchError {
  free(): void;
/**
* @returns {any}
*/
  getInstantLockTransactionId(): any;
/**
* @returns {any}
*/
  getAssetLockTransactionId(): any;
/**
* @returns {number}
*/
  getCode(): number;
/**
* @returns {any}
*/
  serialize(): any;
/**
*/
  readonly message: string;
}
/**
*/
export class IdentityAssetLockTransactionIsNotFoundError {
  free(): void;
/**
* @returns {any}
*/
  getTransactionId(): any;
/**
* @returns {number}
*/
  getCode(): number;
/**
* @returns {any}
*/
  serialize(): any;
/**
*/
  readonly message: string;
}
/**
*/
export class IdentityAssetLockTransactionOutPointAlreadyExistsError {
  free(): void;
/**
* @returns {number}
*/
  getOutputIndex(): number;
/**
* @returns {any}
*/
  getTransactionId(): any;
/**
* @returns {number}
*/
  getCode(): number;
/**
* @returns {any}
*/
  serialize(): any;
/**
*/
  readonly message: string;
}
/**
*/
export class IdentityAssetLockTransactionOutputNotFoundError {
  free(): void;
/**
* @returns {number}
*/
  getOutputIndex(): number;
/**
* @returns {number}
*/
  getCode(): number;
/**
* @returns {any}
*/
  serialize(): any;
/**
*/
  readonly message: string;
}
/**
*/
export class IdentityCreateTransition {
  free(): void;
/**
* @param {any} raw_parameters
*/
  constructor(raw_parameters: any);
/**
* @param {any} asset_lock_proof
*/
  setAssetLockProof(asset_lock_proof: any): void;
/**
* @returns {any}
*/
  getAssetLockProof(): any;
/**
* @param {any[]} public_keys
*/
  setPublicKeys(public_keys: any[]): void;
/**
* @param {any[]} public_keys
*/
  addPublicKeys(public_keys: any[]): void;
/**
* @returns {any[]}
*/
  getPublicKeys(): any[];
/**
* @returns {number}
*/
  getType(): number;
/**
* @returns {any}
*/
  getIdentityId(): any;
/**
* @returns {any}
*/
  getOwnerId(): any;
/**
* @param {any} options
* @returns {any}
*/
  toObject(options: any): any;
/**
* @param {any} options
* @returns {any}
*/
  toBuffer(options: any): any;
/**
* @returns {any}
*/
  toJSON(): any;
/**
* @returns {any[]}
*/
  getModifiedDataIds(): any[];
/**
* @returns {boolean}
*/
  isDataContractStateTransition(): boolean;
/**
* @returns {boolean}
*/
  isDocumentStateTransition(): boolean;
/**
* @returns {boolean}
*/
  isIdentityStateTransition(): boolean;
/**
* @param {StateTransitionExecutionContext} context
*/
  setExecutionContext(context: StateTransitionExecutionContext): void;
/**
* @returns {StateTransitionExecutionContext}
*/
  getExecutionContext(): StateTransitionExecutionContext;
/**
* @param {Uint8Array} private_key
* @param {number} key_type
* @param {any} bls
*/
  signByPrivateKey(private_key: Uint8Array, key_type: number, bls: any): void;
/**
* @returns {any}
*/
  getSignature(): any;
/**
* @param {Uint8Array | undefined} signature
*/
  setSignature(signature?: Uint8Array): void;
/**
*/
  readonly assetLockProof: any;
/**
*/
  readonly identityId: any;
/**
*/
  readonly publicKeys: any[];
}
/**
*/
export class IdentityCreateTransitionBasicValidator {
  free(): void;
/**
* @param {any} state_repository
* @param {any} js_bls
*/
  constructor(state_repository: any, js_bls: any);
/**
* @param {any} raw_state_transition
* @param {StateTransitionExecutionContext} execution_context
* @returns {Promise<ValidationResult>}
*/
  validate(raw_state_transition: any, execution_context: StateTransitionExecutionContext): Promise<ValidationResult>;
}
/**
*/
export class IdentityCreateTransitionStateValidator {
  free(): void;
/**
* @param {any} state_repository
*/
  constructor(state_repository: any);
/**
* @param {IdentityCreateTransition} state_transition
* @returns {Promise<ValidationResult>}
*/
  validate(state_transition: IdentityCreateTransition): Promise<ValidationResult>;
}
/**
*/
export class IdentityFacade {
  free(): void;
/**
* @param {any} asset_lock_proof
* @param {Array<any>} public_keys
* @returns {Identity}
*/
  create(asset_lock_proof: any, public_keys: Array<any>): Identity;
/**
* @param {any} identity_object
* @param {any} options
* @returns {Identity}
*/
  createFromObject(identity_object: any, options: any): Identity;
/**
* @param {Uint8Array} buffer
* @param {any} options
* @returns {Identity}
*/
  createFromBuffer(buffer: Uint8Array, options: any): Identity;
/**
* @param {Identity} identity
* @returns {ValidationResult}
*/
  validate(identity: Identity): ValidationResult;
/**
* @param {Uint8Array} instant_lock
* @param {Uint8Array} asset_lock_transaction
* @param {number} output_index
* @returns {InstantAssetLockProof}
*/
  createInstantAssetLockProof(instant_lock: Uint8Array, asset_lock_transaction: Uint8Array, output_index: number): InstantAssetLockProof;
/**
* @param {number} core_chain_locked_height
* @param {Uint8Array} out_point
* @returns {ChainAssetLockProof}
*/
  createChainAssetLockProof(core_chain_locked_height: number, out_point: Uint8Array): ChainAssetLockProof;
/**
* @param {Identity} identity
* @returns {IdentityCreateTransition}
*/
  createIdentityCreateTransition(identity: Identity): IdentityCreateTransition;
/**
* @param {any} identity_id
* @param {any} asset_lock_proof
* @returns {IdentityTopUpTransition}
*/
  createIdentityTopUpTransition(identity_id: any, asset_lock_proof: any): IdentityTopUpTransition;
/**
* @param {Identity} identity
* @param {any} public_keys
* @returns {IdentityUpdateTransition}
*/
  createIdentityUpdateTransition(identity: Identity, public_keys: any): IdentityUpdateTransition;
}
/**
*/
export class IdentityFactory {
  free(): void;
/**
* @param {number} protocol_version
* @param {IdentityValidator} identity_validator
*/
  constructor(protocol_version: number, identity_validator: IdentityValidator);
/**
* @param {any} asset_lock_proof
* @param {Array<any>} public_keys
* @returns {Identity}
*/
  create(asset_lock_proof: any, public_keys: Array<any>): Identity;
/**
* @param {any} identity_object
* @param {any} options
* @returns {Identity}
*/
  createFromObject(identity_object: any, options: any): Identity;
/**
* @param {Uint8Array} buffer
* @param {any} options
* @returns {Identity}
*/
  createFromBuffer(buffer: Uint8Array, options: any): Identity;
/**
* @param {Uint8Array} instant_lock
* @param {Uint8Array} asset_lock_transaction
* @param {number} output_index
* @returns {InstantAssetLockProof}
*/
  createInstantAssetLockProof(instant_lock: Uint8Array, asset_lock_transaction: Uint8Array, output_index: number): InstantAssetLockProof;
/**
* @param {number} core_chain_locked_height
* @param {Uint8Array} out_point
* @returns {ChainAssetLockProof}
*/
  createChainAssetLockProof(core_chain_locked_height: number, out_point: Uint8Array): ChainAssetLockProof;
/**
* @param {Identity} identity
* @returns {IdentityCreateTransition}
*/
  createIdentityCreateTransition(identity: Identity): IdentityCreateTransition;
/**
* @param {any} identity_id
* @param {any} asset_lock_proof
* @returns {IdentityTopUpTransition}
*/
  createIdentityTopUpTransition(identity_id: any, asset_lock_proof: any): IdentityTopUpTransition;
/**
* @param {Identity} identity
* @param {any} public_keys
* @returns {IdentityUpdateTransition}
*/
  createIdentityUpdateTransition(identity: Identity, public_keys: any): IdentityUpdateTransition;
}
/**
*/
export class IdentityInsufficientBalanceError {
  free(): void;
/**
* @returns {any}
*/
  getIdentityId(): any;
/**
* @returns {number}
*/
  getBalance(): number;
/**
* @returns {number}
*/
  getCode(): number;
/**
* @returns {any}
*/
  serialize(): any;
/**
*/
  readonly message: string;
}
/**
*/
export class IdentityNotFoundError {
  free(): void;
/**
* @param {any} identity_id
*/
  constructor(identity_id: any);
/**
* @returns {any}
*/
  getIdentityId(): any;
/**
* @returns {number}
*/
  getCode(): number;
/**
* @returns {any}
*/
  serialize(): any;
/**
*/
  readonly message: string;
}
/**
*/
export class IdentityPublicKey {
  free(): void;
/**
* @param {any} raw_public_key
*/
  constructor(raw_public_key: any);
/**
* @returns {number}
*/
  getId(): number;
/**
* @param {number} id
*/
  setId(id: number): void;
/**
* @returns {number}
*/
  getType(): number;
/**
* @param {number} key_type
*/
  setType(key_type: number): void;
/**
* @param {Uint8Array} data
*/
  setData(data: Uint8Array): void;
/**
* @returns {any}
*/
  getData(): any;
/**
* @param {number} purpose
*/
  setPurpose(purpose: number): void;
/**
* @returns {number}
*/
  getPurpose(): number;
/**
* @param {number} security_level
*/
  setSecurityLevel(security_level: number): void;
/**
* @returns {number}
*/
  getSecurityLevel(): number;
/**
* @param {boolean} read_only
*/
  setReadOnly(read_only: boolean): void;
/**
* @returns {boolean}
*/
  isReadOnly(): boolean;
/**
* @param {Date} timestamp
*/
  setDisabledAt(timestamp: Date): void;
/**
* @returns {Date | undefined}
*/
  getDisabledAt(): Date | undefined;
/**
* @returns {Uint8Array}
*/
  hash(): Uint8Array;
/**
* @returns {boolean}
*/
  isMaster(): boolean;
/**
* @returns {any}
*/
  toJSON(): any;
/**
* @returns {any}
*/
  toObject(): any;
}
/**
*/
export class IdentityPublicKeyDisabledAtWindowViolationError {
  free(): void;
/**
* @returns {Date}
*/
  getDisabledAt(): Date;
/**
* @returns {Date}
*/
  getTimeWindowStart(): Date;
/**
* @returns {Date}
*/
  getTimeWindowEnd(): Date;
/**
* @returns {number}
*/
  getCode(): number;
/**
* @returns {any}
*/
  serialize(): any;
/**
*/
  readonly message: string;
}
/**
*/
export class IdentityPublicKeyIsDisabledError {
  free(): void;
/**
* @returns {number}
*/
  getPublicKeyIndex(): number;
/**
* @returns {number}
*/
  getCode(): number;
/**
* @returns {any}
*/
  serialize(): any;
/**
*/
  readonly message: string;
}
/**
*/
export class IdentityPublicKeyIsReadOnlyError {
  free(): void;
/**
* @returns {number}
*/
  getKeyId(): number;
/**
* @returns {number}
*/
  getPublicKeyIndex(): number;
/**
* @returns {number}
*/
  getCode(): number;
/**
* @returns {any}
*/
  serialize(): any;
/**
*/
  readonly message: string;
}
/**
*/
export class IdentityPublicKeyWithWitness {
  free(): void;
/**
* @param {any} raw_public_key
*/
  constructor(raw_public_key: any);
/**
* @returns {number}
*/
  getId(): number;
/**
* @param {number} id
*/
  setId(id: number): void;
/**
* @returns {number}
*/
  getType(): number;
/**
* @param {number} key_type
*/
  setType(key_type: number): void;
/**
* @param {Uint8Array} data
*/
  setData(data: Uint8Array): void;
/**
* @returns {any}
*/
  getData(): any;
/**
* @param {number} purpose
*/
  setPurpose(purpose: number): void;
/**
* @returns {number}
*/
  getPurpose(): number;
/**
* @param {number} security_level
*/
  setSecurityLevel(security_level: number): void;
/**
* @returns {number}
*/
  getSecurityLevel(): number;
/**
* @param {boolean} read_only
*/
  setReadOnly(read_only: boolean): void;
/**
* @returns {boolean}
*/
  isReadOnly(): boolean;
/**
* @param {Uint8Array} signature
*/
  setSignature(signature: Uint8Array): void;
/**
* @returns {Uint8Array}
*/
  getSignature(): Uint8Array;
/**
* @returns {Uint8Array}
*/
  hash(): Uint8Array;
/**
* @returns {boolean}
*/
  isMaster(): boolean;
/**
* @returns {any}
*/
  toJSON(): any;
/**
* @param {any} options
* @returns {any}
*/
  toObject(options: any): any;
}
/**
*/
export class IdentityTopUpTransition {
  free(): void;
/**
* @param {any} raw_parameters
*/
  constructor(raw_parameters: any);
/**
* @param {any} asset_lock_proof
*/
  setAssetLockProof(asset_lock_proof: any): void;
/**
* @returns {any}
*/
  getAssetLockProof(): any;
/**
* @returns {number}
*/
  getType(): number;
/**
* @returns {any}
*/
  getIdentityId(): any;
/**
* @returns {any}
*/
  getOwnerId(): any;
/**
* @param {any} options
* @returns {any}
*/
  toObject(options: any): any;
/**
* @param {any} options
* @returns {any}
*/
  toBuffer(options: any): any;
/**
* @returns {any}
*/
  toJSON(): any;
/**
* @returns {any[]}
*/
  getModifiedDataIds(): any[];
/**
* @returns {boolean}
*/
  isDataContractStateTransition(): boolean;
/**
* @returns {boolean}
*/
  isDocumentStateTransition(): boolean;
/**
* @returns {boolean}
*/
  isIdentityStateTransition(): boolean;
/**
* @param {StateTransitionExecutionContext} context
*/
  setExecutionContext(context: StateTransitionExecutionContext): void;
/**
* @returns {StateTransitionExecutionContext}
*/
  getExecutionContext(): StateTransitionExecutionContext;
/**
* @param {Uint8Array} private_key
* @param {number} key_type
* @param {any} bls
*/
  signByPrivateKey(private_key: Uint8Array, key_type: number, bls: any): void;
/**
* @returns {any}
*/
  getSignature(): any;
/**
* @param {Uint8Array | undefined} signature
*/
  setSignature(signature?: Uint8Array): void;
/**
*/
  readonly assetLockProof: any;
/**
*/
  readonly identityId: any;
}
/**
*/
export class IdentityTopUpTransitionBasicValidator {
  free(): void;
/**
* @param {any} state_repository
*/
  constructor(state_repository: any);
/**
* @param {any} raw_state_transition
* @param {StateTransitionExecutionContext} execution_context
* @returns {Promise<ValidationResult>}
*/
  validate(raw_state_transition: any, execution_context: StateTransitionExecutionContext): Promise<ValidationResult>;
}
/**
*/
export class IdentityTopUpTransitionStateValidator {
  free(): void;
/**
* @param {any} state_repository
*/
  constructor(state_repository: any);
/**
* @param {IdentityTopUpTransition} state_transition
* @returns {Promise<ValidationResult>}
*/
  validate(state_transition: IdentityTopUpTransition): Promise<ValidationResult>;
}
/**
*/
export class IdentityUpdatePublicKeysValidator {
  free(): void;
/**
*/
  constructor();
/**
* @param {any[]} raw_public_keys
* @returns {ValidationResult}
*/
  validate(raw_public_keys: any[]): ValidationResult;
}
/**
*/
export class IdentityUpdateTransition {
  free(): void;
/**
* @param {any} raw_parameters
*/
  constructor(raw_parameters: any);
/**
* @param {any[] | undefined} public_keys
*/
  setPublicKeysToAdd(public_keys?: any[]): void;
/**
* @returns {any[]}
*/
  getPublicKeysToAdd(): any[];
/**
* @returns {any[]}
*/
  getPublicKeyIdsToDisable(): any[];
/**
* @param {Uint32Array | undefined} public_key_ids
*/
  setPublicKeyIdsToDisable(public_key_ids?: Uint32Array): void;
/**
* @returns {Date | undefined}
*/
  getPublicKeysDisabledAt(): Date | undefined;
/**
* @param {Date | undefined} timestamp
*/
  setPublicKeysDisabledAt(timestamp?: Date): void;
/**
* @returns {number}
*/
  getType(): number;
/**
* @returns {any}
*/
  getIdentityId(): any;
/**
* @param {any} identity_id
*/
  setIdentityId(identity_id: any): void;
/**
* @returns {any}
*/
  getOwnerId(): any;
/**
* @param {any} options
* @returns {any}
*/
  toObject(options: any): any;
/**
* @param {any} options
* @returns {any}
*/
  toBuffer(options: any): any;
/**
* @returns {any}
*/
  toJSON(): any;
/**
* @returns {any[]}
*/
  getModifiedDataIds(): any[];
/**
* @returns {boolean}
*/
  isDataContractStateTransition(): boolean;
/**
* @returns {boolean}
*/
  isDocumentStateTransition(): boolean;
/**
* @returns {boolean}
*/
  isIdentityStateTransition(): boolean;
/**
* @param {StateTransitionExecutionContext} context
*/
  setExecutionContext(context: StateTransitionExecutionContext): void;
/**
* @returns {StateTransitionExecutionContext}
*/
  getExecutionContext(): StateTransitionExecutionContext;
/**
* @param {Uint8Array} private_key
* @param {number} key_type
* @param {any} bls
*/
  signByPrivateKey(private_key: Uint8Array, key_type: number, bls: any): void;
/**
* @param {number | undefined} key_id
*/
  setSignaturePublicKeyId(key_id?: number): void;
/**
* @returns {any}
*/
  getSignature(): any;
/**
* @param {Uint8Array | undefined} signature
*/
  setSignature(signature?: Uint8Array): void;
/**
* @returns {number}
*/
  getRevision(): number;
/**
* @param {number} revision
*/
  setRevision(revision: number): void;
/**
* @param {IdentityPublicKey} identity_public_key
* @param {Uint8Array} private_key
* @param {any} bls
*/
  sign(identity_public_key: IdentityPublicKey, private_key: Uint8Array, bls: any): void;
/**
* @param {IdentityPublicKey} identity_public_key
* @param {any} bls
* @returns {boolean}
*/
  verifySignature(identity_public_key: IdentityPublicKey, bls: any): boolean;
/**
*/
  readonly addPublicKeys: any[];
/**
*/
  readonly identityId: any;
}
/**
*/
export class IdentityUpdateTransitionBasicValidator {
  free(): void;
/**
* @param {any} js_bls
*/
  constructor(js_bls: any);
/**
* @param {any} raw_state_transition
* @returns {ValidationResult}
*/
  validate(raw_state_transition: any): ValidationResult;
}
/**
*/
export class IdentityUpdateTransitionStateValidator {
  free(): void;
/**
* @param {any} state_repository
*/
  constructor(state_repository: any);
/**
* @param {IdentityUpdateTransition} state_transition
* @returns {Promise<ValidationResult>}
*/
  validate(state_transition: IdentityUpdateTransition): Promise<ValidationResult>;
}
/**
*/
export class IdentityValidator {
  free(): void;
/**
* @param {any} bls
*/
  constructor(bls: any);
/**
* @param {any} raw_identity
* @returns {ValidationResult}
*/
  validate(raw_identity: any): ValidationResult;
}
/**
*/
export class IncompatibleDataContractSchemaError {
  free(): void;
/**
* @returns {any}
*/
  getDataContractId(): any;
/**
* @returns {string}
*/
  getOperation(): string;
/**
* @returns {string}
*/
  getFieldPath(): string;
/**
* @returns {number}
*/
  getCode(): number;
/**
* @returns {any}
*/
  serialize(): any;
/**
*/
  readonly message: string;
}
/**
*/
export class IncompatibleProtocolVersionError {
  free(): void;
/**
* @returns {number}
*/
  getParsedProtocolVersion(): number;
/**
* @returns {number}
*/
  getMinimalProtocolVersion(): number;
/**
* @returns {number}
*/
  getCode(): number;
/**
* @returns {any}
*/
  serialize(): any;
/**
*/
  readonly message: string;
}
/**
*/
export class IncompatibleRe2PatternError {
  free(): void;
/**
* @returns {string}
*/
  getPattern(): string;
/**
* @returns {string}
*/
  getPath(): string;
/**
* @returns {string}
*/
  getMessage(): string;
/**
* @returns {number}
*/
  getCode(): number;
/**
* @returns {any}
*/
  serialize(): any;
/**
*/
  readonly message: string;
}
/**
*/
export class InconsistentCompoundIndexDataError {
  free(): void;
/**
* @returns {Array<any>}
*/
  getIndexedProperties(): Array<any>;
/**
* @returns {string}
*/
  getDocumentType(): string;
/**
* @returns {number}
*/
  getCode(): number;
/**
* @returns {any}
*/
  serialize(): any;
/**
*/
  readonly message: string;
}
/**
*/
export class IndexDefinition {
  free(): void;
/**
* @returns {string}
*/
  getName(): string;
/**
* @returns {any[]}
*/
  getProperties(): any[];
/**
* @returns {boolean}
*/
  isUnique(): boolean;
/**
* @returns {any}
*/
  toObject(): any;
}
/**
*/
export class IndexProperty {
  free(): void;
/**
* @returns {string}
*/
  getName(): string;
/**
* @returns {boolean}
*/
  isAscending(): boolean;
}
/**
*/
export class InstantAssetLockProof {
  free(): void;
/**
* @param {any} raw_parameters
*/
  constructor(raw_parameters: any);
/**
* @returns {number}
*/
  getType(): number;
/**
* @returns {number}
*/
  getOutputIndex(): number;
/**
* @returns {any | undefined}
*/
  getOutPoint(): any | undefined;
/**
* @returns {any}
*/
  getOutput(): any;
/**
* @returns {any}
*/
  createIdentifier(): any;
/**
* @returns {any}
*/
  getInstantLock(): any;
/**
* @returns {any}
*/
  getTransaction(): any;
/**
* @returns {any}
*/
  toObject(): any;
/**
* @returns {any}
*/
  toJSON(): any;
}
/**
*/
export class InstantAssetLockProofStructureValidator {
  free(): void;
/**
* @param {any} state_repository
*/
  constructor(state_repository: any);
/**
* @param {any} raw_asset_lock_proof
* @param {StateTransitionExecutionContext} execution_context
* @returns {Promise<ValidationResult>}
*/
  validate(raw_asset_lock_proof: any, execution_context: StateTransitionExecutionContext): Promise<ValidationResult>;
}
/**
*/
export class InvalidActionError {
  free(): void;
}
/**
*/
export class InvalidActionNameError {
  free(): void;
/**
* @param {any[]} actions
*/
  constructor(actions: any[]);
/**
* @returns {any[]}
*/
  getActions(): any[];
}
/**
*/
export class InvalidAssetLockProofCoreChainHeightError {
  free(): void;
/**
* @returns {number}
*/
  getProofCoreChainLockedHeight(): number;
/**
* @returns {number}
*/
  getCurrentCoreChainLockedHeight(): number;
/**
* @returns {number}
*/
  getCode(): number;
/**
* @returns {any}
*/
  serialize(): any;
/**
*/
  readonly message: string;
}
/**
*/
export class InvalidAssetLockProofTransactionHeightError {
  free(): void;
/**
* @returns {number}
*/
  getProofCoreChainLockedHeight(): number;
/**
* @returns {number | undefined}
*/
  getTransactionHeight(): number | undefined;
/**
* @returns {number}
*/
  getCode(): number;
/**
* @returns {any}
*/
  serialize(): any;
/**
*/
  readonly message: string;
}
/**
*/
export class InvalidAssetLockTransactionOutputReturnSizeError {
  free(): void;
/**
* @returns {number}
*/
  getOutputIndex(): number;
/**
* @returns {number}
*/
  getCode(): number;
/**
* @returns {any}
*/
  serialize(): any;
/**
*/
  readonly message: string;
}
/**
*/
export class InvalidCompoundIndexError {
  free(): void;
/**
* @returns {string}
*/
  getDocumentType(): string;
/**
* @returns {string}
*/
  getIndexName(): string;
/**
* @returns {number}
*/
  getCode(): number;
/**
* @returns {any}
*/
  serialize(): any;
/**
*/
  readonly message: string;
}
/**
*/
export class InvalidDataContractError {
  free(): void;
/**
* @returns {any[]}
*/
  getErrors(): any[];
/**
* @returns {any}
*/
  getRawDataContract(): any;
/**
* @returns {string}
*/
  getMessage(): string;
}
/**
*/
export class InvalidDataContractIdError {
  free(): void;
/**
* @returns {any}
*/
  getExpectedId(): any;
/**
* @returns {any}
*/
  getInvalidId(): any;
/**
* @returns {number}
*/
  getCode(): number;
/**
* @returns {any}
*/
  serialize(): any;
/**
*/
  readonly message: string;
}
/**
*/
export class InvalidDataContractVersionError {
  free(): void;
/**
* @returns {number}
*/
  getExpectedVersion(): number;
/**
* @returns {number}
*/
  getVersion(): number;
/**
* @returns {number}
*/
  getCode(): number;
/**
* @returns {any}
*/
  serialize(): any;
/**
*/
  readonly message: string;
}
/**
*/
export class InvalidDocumentActionError {
  free(): void;
/**
* @returns {any}
*/
  getDocumentTransition(): any;
}
/**
*/
export class InvalidDocumentError {
  free(): void;
/**
* @param {any} raw_document
* @param {any[]} errors
*/
  constructor(raw_document: any, errors: any[]);
/**
* @returns {any[]}
*/
  getErrors(): any[];
/**
* @returns {any}
*/
  getRawDocument(): any;
}
/**
*/
export class InvalidDocumentRevisionError {
  free(): void;
/**
* @returns {any}
*/
  getDocumentId(): any;
/**
* @returns {bigint | undefined}
*/
  getCurrentRevision(): bigint | undefined;
/**
* @returns {number}
*/
  getCode(): number;
/**
* @returns {any}
*/
  serialize(): any;
/**
*/
  readonly message: string;
}
/**
*/
export class InvalidDocumentTransitionActionError {
  free(): void;
/**
* @returns {string}
*/
  getAction(): string;
/**
* @returns {number}
*/
  getCode(): number;
/**
* @returns {any}
*/
  serialize(): any;
/**
*/
  readonly message: string;
}
/**
*/
export class InvalidDocumentTransitionIdError {
  free(): void;
/**
* @returns {any}
*/
  getExpectedId(): any;
/**
* @returns {any}
*/
  getInvalidId(): any;
/**
* @returns {number}
*/
  getCode(): number;
/**
* @returns {any}
*/
  serialize(): any;
/**
*/
  readonly message: string;
}
/**
*/
export class InvalidDocumentTypeError {
  free(): void;
/**
* @returns {string}
*/
  getType(): string;
/**
* @returns {any}
*/
  getDataContractId(): any;
/**
* @returns {number}
*/
  getCode(): number;
/**
* @returns {any}
*/
  serialize(): any;
/**
*/
  readonly message: string;
}
/**
*/
export class InvalidDocumentTypeInDataContractError {
  free(): void;
/**
* @param {string} doc_type
* @param {any} data_contract_id
*/
  constructor(doc_type: string, data_contract_id: any);
/**
* @returns {string}
*/
  getType(): string;
/**
* @returns {any}
*/
  getDataContractId(): any;
}
/**
*/
export class InvalidIdentifierError {
  free(): void;
/**
* @returns {string}
*/
  getIdentifierName(): string;
/**
* @returns {string}
*/
  getIdentifierError(): string;
/**
* @returns {number}
*/
  getCode(): number;
/**
* @returns {any}
*/
  serialize(): any;
/**
*/
  readonly message: string;
}
/**
*/
export class InvalidIdentityAssetLockTransactionError {
  free(): void;
/**
* @returns {string}
*/
  getErrorMessage(): string;
/**
* @returns {number}
*/
  getCode(): number;
/**
* @returns {any}
*/
  serialize(): any;
/**
*/
  readonly message: string;
}
/**
*/
export class InvalidIdentityAssetLockTransactionOutputError {
  free(): void;
/**
* @returns {number}
*/
  getOutputIndex(): number;
/**
* @returns {number}
*/
  getCode(): number;
/**
* @returns {any}
*/
  serialize(): any;
/**
*/
  readonly message: string;
}
/**
*/
export class InvalidIdentityCreditWithdrawalTransitionCoreFeeError {
  free(): void;
/**
* @returns {number}
*/
  getCoreFee(): number;
/**
* @returns {number}
*/
  getCode(): number;
/**
* @returns {any}
*/
  serialize(): any;
/**
*/
  readonly message: string;
}
/**
*/
export class InvalidIdentityCreditWithdrawalTransitionOutputScriptError {
  free(): void;
/**
* @returns {number}
*/
  getCode(): number;
/**
* @returns {any}
*/
  serialize(): any;
/**
*/
  readonly message: string;
}
/**
*/
export class InvalidIdentityError {
  free(): void;
/**
* @returns {any[]}
*/
  getErrors(): any[];
/**
* @returns {any}
*/
  getRawIdentity(): any;
}
/**
*/
export class InvalidIdentityKeySignatureError {
  free(): void;
/**
* @returns {number}
*/
  getPublicKeyId(): number;
/**
* @returns {number}
*/
  getCode(): number;
/**
* @returns {any}
*/
  serialize(): any;
/**
*/
  readonly message: string;
}
/**
*/
export class InvalidIdentityPublicKeyDataError {
  free(): void;
/**
* @returns {number}
*/
  getPublicKeyId(): number;
/**
* @returns {string}
*/
  getValidationError(): string;
/**
* @returns {number}
*/
  getCode(): number;
/**
* @returns {any}
*/
  serialize(): any;
/**
*/
  readonly message: string;
}
/**
*/
export class InvalidIdentityPublicKeyIdError {
  free(): void;
/**
* @returns {number}
*/
  getId(): number;
/**
* @returns {number}
*/
  getCode(): number;
/**
* @returns {any}
*/
  serialize(): any;
/**
*/
  readonly message: string;
}
/**
*/
export class InvalidIdentityPublicKeySecurityLevelError {
  free(): void;
/**
* @returns {number}
*/
  getPublicKeyId(): number;
/**
* @returns {number}
*/
  getPublicKeyPurpose(): number;
/**
* @returns {number}
*/
  getPublicKeySecurityLevel(): number;
/**
* @returns {number}
*/
  getCode(): number;
/**
* @returns {any}
*/
  serialize(): any;
/**
*/
  readonly message: string;
}
/**
*/
export class InvalidIdentityPublicKeyTypeError {
  free(): void;
/**
* @param {number} key_type
*/
  constructor(key_type: number);
/**
* @returns {number}
*/
  getPublicKeyType(): number;
/**
* @returns {number}
*/
  getCode(): number;
/**
* @returns {any}
*/
  serialize(): any;
/**
*/
  readonly message: string;
}
/**
*/
export class InvalidIdentityRevisionError {
  free(): void;
/**
* @returns {any}
*/
  getIdentityId(): any;
/**
* @returns {number}
*/
  getCurrentRevision(): number;
/**
* @returns {number}
*/
  getCode(): number;
/**
* @returns {any}
*/
  serialize(): any;
/**
*/
  readonly message: string;
}
/**
*/
export class InvalidIndexPropertyTypeError {
  free(): void;
/**
* @returns {string}
*/
  getDocumentType(): string;
/**
* @returns {string}
*/
  getIndexName(): string;
/**
* @returns {string}
*/
  getPropertyName(): string;
/**
* @returns {string}
*/
  getPropertyType(): string;
/**
* @returns {number}
*/
  getCode(): number;
/**
* @returns {any}
*/
  serialize(): any;
/**
*/
  readonly message: string;
}
/**
*/
export class InvalidIndexedPropertyConstraintError {
  free(): void;
/**
* @returns {string}
*/
  getDocumentType(): string;
/**
* @returns {string}
*/
  getIndexName(): string;
/**
* @returns {string}
*/
  getPropertyName(): string;
/**
* @returns {string}
*/
  getConstraintName(): string;
/**
* @returns {string}
*/
  getReason(): string;
/**
* @returns {number}
*/
  getCode(): number;
/**
* @returns {any}
*/
  serialize(): any;
/**
*/
  readonly message: string;
}
/**
*/
export class InvalidInitialRevisionError {
  free(): void;
/**
* @param {ExtendedDocument} document
*/
  constructor(document: ExtendedDocument);
/**
* @returns {ExtendedDocument}
*/
  getDocument(): ExtendedDocument;
}
/**
*/
export class InvalidInstantAssetLockProofError {
  free(): void;
/**
* @returns {number}
*/
  getCode(): number;
/**
* @returns {any}
*/
  serialize(): any;
/**
*/
  readonly message: string;
}
/**
*/
export class InvalidInstantAssetLockProofSignatureError {
  free(): void;
/**
* @returns {number}
*/
  getCode(): number;
/**
* @returns {any}
*/
  serialize(): any;
/**
*/
  readonly message: string;
}
/**
*/
export class InvalidJsonSchemaRefError {
  free(): void;
/**
* @returns {string}
*/
  getRefError(): string;
/**
* @returns {number}
*/
  getCode(): number;
/**
* @returns {any}
*/
  serialize(): any;
/**
*/
  readonly message: string;
}
/**
*/
export class InvalidSignaturePublicKeySecurityLevelError {
  free(): void;
/**
* @returns {number}
*/
  getPublicKeySecurityLevel(): number;
/**
* @returns {number}
*/
  getKeySecurityLevelRequirement(): number;
/**
* @returns {number}
*/
  getCode(): number;
/**
* @returns {any}
*/
  serialize(): any;
/**
*/
  readonly message: string;
}
/**
*/
export class InvalidStateTransitionError {
  free(): void;
/**
* @param {any[]} error_buffers
* @param {any} raw_state_transition
*/
  constructor(error_buffers: any[], raw_state_transition: any);
/**
* @returns {any[]}
*/
  getErrors(): any[];
/**
* @returns {any}
*/
  getRawStateTransition(): any;
}
/**
*/
export class InvalidStateTransitionSignatureError {
  free(): void;
/**
* @returns {number}
*/
  getCode(): number;
/**
* @returns {any}
*/
  serialize(): any;
/**
*/
  readonly message: string;
}
/**
*/
export class InvalidStateTransitionTypeError {
  free(): void;
/**
* @param {number} transition_type
*/
  constructor(transition_type: number);
/**
* @returns {number}
*/
  getType(): number;
/**
* @returns {number}
*/
  getCode(): number;
/**
* @returns {any}
*/
  serialize(): any;
/**
*/
  readonly message: string;
}
/**
*/
export class JsonSchemaCompilationError {
  free(): void;
/**
* @returns {string}
*/
  getError(): string;
/**
* @returns {number}
*/
  getCode(): number;
/**
* @returns {any}
*/
  serialize(): any;
/**
*/
  readonly message: string;
}
/**
*/
export class JsonSchemaError {
/**
** Return copy of self without private attributes.
*/
  toJSON(): Object;
/**
* Return stringified version of self.
*/
  toString(): string;
  free(): void;
/**
* @returns {string}
*/
  getKeyword(): string;
/**
* @returns {string}
*/
  getInstancePath(): string;
/**
* @returns {string}
*/
  getSchemaPath(): string;
/**
* @returns {string}
*/
  getPropertyName(): string;
/**
* @returns {any}
*/
  getParams(): any;
/**
* @returns {number}
*/
  getCode(): number;
/**
* @returns {string}
*/
  toString(): string;
/**
* @returns {any}
*/
  serialize(): any;
/**
*/
  readonly message: string;
}
/**
*/
export class JsonSchemaValidator {
  free(): void;
/**
* @param {any} schema_js
* @param {any} definitions
*/
  constructor(schema_js: any, definitions: any);
}
/**
*/
export class MaxIdentityPublicKeyLimitReachedError {
  free(): void;
/**
* @returns {number}
*/
  getMaxItems(): number;
/**
* @returns {number}
*/
  getCode(): number;
/**
* @returns {any}
*/
  serialize(): any;
/**
*/
  readonly message: string;
}
/**
*/
export class Metadata {
  free(): void;
/**
* @param {any} options
*/
  constructor(options: any);
/**
* @param {any} object
* @returns {Metadata}
*/
  static from(object: any): Metadata;
/**
* @returns {any}
*/
  toJSON(): any;
/**
* @returns {any}
*/
  toObject(): any;
/**
* @returns {number}
*/
  getBlockHeight(): number;
/**
* @returns {number}
*/
  getCoreChainLockedHeight(): number;
/**
* @returns {number}
*/
  getTimeMs(): number;
/**
* @returns {number}
*/
  getProtocolVersion(): number;
}
/**
*/
export class MismatchOwnerIdsError {
  free(): void;
/**
* @param {any[]} documents
*/
  constructor(documents: any[]);
/**
* @returns {any[]}
*/
  getDocuments(): any[];
}
/**
*/
export class MissingDataContractIdError {
  free(): void;
/**
* @returns {number}
*/
  getCode(): number;
/**
* @returns {any}
*/
  serialize(): any;
/**
*/
  readonly message: string;
}
/**
*/
export class MissingDocumentTransitionActionError {
  free(): void;
/**
* @returns {number}
*/
  getCode(): number;
/**
* @returns {any}
*/
  serialize(): any;
/**
*/
  readonly message: string;
}
/**
*/
export class MissingDocumentTransitionTypeError {
  free(): void;
/**
* @returns {number}
*/
  getCode(): number;
/**
* @returns {any}
*/
  serialize(): any;
/**
*/
  readonly message: string;
}
/**
*/
export class MissingDocumentTypeError {
  free(): void;
/**
* @returns {number}
*/
  getCode(): number;
/**
* @returns {any}
*/
  serialize(): any;
/**
*/
  readonly message: string;
}
/**
*/
export class MissingMasterPublicKeyError {
  free(): void;
/**
* @returns {number}
*/
  getCode(): number;
/**
* @returns {any}
*/
  serialize(): any;
/**
*/
  readonly message: string;
}
/**
*/
export class MissingPublicKeyError {
  free(): void;
/**
* @returns {number}
*/
  getPublicKeyId(): number;
/**
* @returns {number}
*/
  getCode(): number;
/**
* @returns {any}
*/
  serialize(): any;
/**
*/
  readonly message: string;
}
/**
*/
export class MissingStateTransitionTypeError {
  free(): void;
/**
*/
  constructor();
/**
* @returns {number}
*/
  getCode(): number;
/**
* @returns {any}
*/
  serialize(): any;
/**
*/
  readonly message: string;
}
/**
*/
export class NoDocumentsSuppliedError {
  free(): void;
/**
*/
  constructor();
}
/**
*/
export class NonConsensusErrorWasm {
  free(): void;
}
/**
*/
export class NotImplementedIdentityCreditWithdrawalTransitionPoolingError {
  free(): void;
/**
* @returns {number}
*/
  getPooling(): number;
/**
* @returns {number}
*/
  getCode(): number;
/**
* @returns {any}
*/
  serialize(): any;
/**
*/
  readonly message: string;
}
/**
*/
export class Operation {
  free(): void;
}
/**
*/
export class PlatformValueError {
/**
** Return copy of self without private attributes.
*/
  toJSON(): Object;
/**
* Return stringified version of self.
*/
  toString(): string;
  free(): void;
/**
* @returns {string}
*/
  getMessage(): string;
/**
* @returns {string}
*/
  toString(): string;
}
/**
*/
export class PreCalculatedOperation {
  free(): void;
/**
* @param {any} storage_cost
* @param {any} processing_cost
* @param {Array<any>} js_fee_refunds
*/
  constructor(storage_cost: any, processing_cost: any, js_fee_refunds: Array<any>);
/**
* @param {DummyFeesResult} dummy_fee_result
* @returns {PreCalculatedOperation}
*/
  static fromFee(dummy_fee_result: DummyFeesResult): PreCalculatedOperation;
/**
* @returns {bigint}
*/
  getProcessingCost(): bigint;
/**
* @returns {bigint}
*/
  getStorageCost(): bigint;
/**
* @returns {Array<any> | undefined}
*/
  refunds_as_objects(): Array<any> | undefined;
/**
* @returns {any}
*/
  toJSON(): any;
/**
*/
  readonly refunds: Array<any> | undefined;
}
/**
*/
export class ProtocolVersionParsingError {
  free(): void;
/**
* @param {string} parsing_error
*/
  constructor(parsing_error: string);
/**
* @returns {string}
*/
  getParsingError(): string;
/**
* @returns {number}
*/
  getCode(): number;
/**
* @returns {any}
*/
  serialize(): any;
/**
*/
  readonly message: string;
}
/**
*/
export class ProtocolVersionValidator {
  free(): void;
/**
* @param {any} options
*/
  constructor(options: any);
/**
* @param {number} version
* @returns {ValidationResult}
*/
  validate(version: number): ValidationResult;
}
/**
*/
export class PublicKeyIsDisabledError {
  free(): void;
/**
* @returns {number}
*/
  getPublicKeyId(): number;
/**
* @returns {number}
*/
  getCode(): number;
/**
* @returns {any}
*/
  serialize(): any;
/**
*/
  readonly message: string;
}
/**
*/
export class PublicKeySecurityLevelNotMetError {
  free(): void;
/**
* @returns {number}
*/
  getPublicKeySecurityLevel(): number;
/**
* @returns {number}
*/
  getKeySecurityLevelRequirement(): number;
/**
* @returns {number}
*/
  getCode(): number;
/**
* @returns {any}
*/
  serialize(): any;
/**
*/
  readonly message: string;
}
/**
*/
export class PublicKeyValidationError {
  free(): void;
/**
*/
  readonly message: any;
}
/**
*/
export class PublicKeysSignaturesValidator {
  free(): void;
/**
* @param {any} bls
*/
  constructor(bls: any);
/**
* @param {any} raw_state_transition
* @param {any[]} raw_public_keys
* @returns {ValidationResult}
*/
  validate(raw_state_transition: any, raw_public_keys: any[]): ValidationResult;
}
/**
*/
export class PublicKeysValidator {
  free(): void;
/**
* @param {any} adapter
*/
  constructor(adapter: any);
/**
* @param {Array<any>} public_keys
* @returns {ValidationResult}
*/
  validateKeys(public_keys: Array<any>): ValidationResult;
/**
* @param {any} public_key
* @returns {ValidationResult}
*/
  validatePublicKeyStructure(public_key: any): ValidationResult;
/**
* @param {Array<any>} public_keys
* @returns {ValidationResult}
*/
  validateKeysInStateTransition(public_keys: Array<any>): ValidationResult;
}
/**
*/
export class ReadOperation {
  free(): void;
/**
* @param {any} value_size
*/
  constructor(value_size: any);
/**
* @returns {any}
*/
  toJSON(): any;
/**
*/
  readonly processingCost: bigint;
/**
*/
  readonly refunds: Array<any> | undefined;
/**
*/
  readonly storageCost: bigint;
}
/**
*/
export class Refunds {
  free(): void;
/**
* @returns {any}
*/
  toObject(): any;
/**
*/
  readonly credits_per_epoch: Map<any, any>;
/**
*/
  readonly identifier: any;
}
/**
*/
export class RevisionAbsentError {
  free(): void;
/**
* @param {ExtendedDocument} extended_document
*/
  constructor(extended_document: ExtendedDocument);
/**
* @returns {ExtendedDocument}
*/
  getDocument(): ExtendedDocument;
}
/**
*/
export class SerializedObjectParsingError {
  free(): void;
/**
* @returns {string}
*/
  getParsingError(): string;
/**
* @returns {number}
*/
  getCode(): number;
/**
* @returns {any}
*/
  serialize(): any;
/**
*/
  readonly message: string;
}
/**
*/
export class SignatureVerificationOperation {
  free(): void;
/**
* @param {number} signature_type
*/
  constructor(signature_type: number);
/**
* @returns {bigint}
*/
  getProcessingCost(): bigint;
/**
* @returns {bigint}
*/
  getStorageCost(): bigint;
/**
* @returns {any}
*/
  toJSON(): any;
/**
*/
  readonly refunds: Array<any> | undefined;
}
/**
*/
export class StateTransitionExecutionContext {
  free(): void;
/**
*/
  constructor();
/**
*/
  enableDryRun(): void;
/**
*/
  disableDryRun(): void;
/**
* @returns {boolean}
*/
  isDryRun(): boolean;
/**
* @param {any} operation
*/
  addOperation(operation: any): void;
/**
* @returns {any[]}
*/
  getOperations(): any[];
/**
*/
  clearDryOperations(): void;
}
/**
*/
export class StateTransitionFacade {
  free(): void;
/**
* @param {any} raw_state_transition
* @param {any} options
* @returns {Promise<any>}
*/
  createFromObject(raw_state_transition: any, options: any): Promise<any>;
/**
* @param {Uint8Array} state_transition_buffer
* @param {any} options
* @returns {Promise<any>}
*/
  createFromBuffer(state_transition_buffer: Uint8Array, options: any): Promise<any>;
/**
* @param {any} raw_state_transition
* @param {any} options
* @returns {Promise<ValidationResult>}
*/
  validate(raw_state_transition: any, options: any): Promise<ValidationResult>;
/**
* @param {any} raw_state_transition
* @returns {Promise<ValidationResult>}
*/
  validateBasic(raw_state_transition: any): Promise<ValidationResult>;
/**
* @param {any} raw_state_transition
* @returns {Promise<ValidationResult>}
*/
  validateSignature(raw_state_transition: any): Promise<ValidationResult>;
/**
* @param {any} raw_state_transition
* @returns {Promise<ValidationResult>}
*/
  validateFee(raw_state_transition: any): Promise<ValidationResult>;
/**
* @param {any} raw_state_transition
* @returns {Promise<ValidationResult>}
*/
  validateState(raw_state_transition: any): Promise<ValidationResult>;
/**
* @param {any} state_transition_js
* @returns {Promise<any>}
*/
  apply(state_transition_js: any): Promise<any>;
}
/**
*/
export class StateTransitionFactory {
  free(): void;
/**
* @param {any} state_repository
* @param {any} bls_adapter
*/
  constructor(state_repository: any, bls_adapter: any);
/**
* @param {any} state_transition_object
* @param {any} options
* @returns {Promise<any>}
*/
  createFromObject(state_transition_object: any, options: any): Promise<any>;
/**
* @param {Uint8Array} buffer
* @param {any} options
* @returns {Promise<any>}
*/
  createFromBuffer(buffer: Uint8Array, options: any): Promise<any>;
}
/**
*/
export class StateTransitionKeySignatureValidator {
  free(): void;
/**
* @param {any} state_repository
*/
  constructor(state_repository: any);
/**
* @param {any} state_transition
* @returns {Promise<ValidationResult>}
*/
  validate(state_transition: any): Promise<ValidationResult>;
}
/**
*/
export class StateTransitionMaxSizeExceededError {
  free(): void;
/**
* @returns {number}
*/
  getActualSizeKBytes(): number;
/**
* @returns {number}
*/
  getMaxSizeKBytes(): number;
/**
* @returns {number}
*/
  getCode(): number;
/**
* @returns {any}
*/
  serialize(): any;
/**
*/
  readonly message: string;
}
/**
*/
export class SystemPropertyIndexAlreadyPresentError {
  free(): void;
/**
* @returns {string}
*/
  getDocumentType(): string;
/**
* @returns {string}
*/
  getIndexName(): string;
/**
* @returns {string}
*/
  getPropertyName(): string;
/**
* @returns {number}
*/
  getCode(): number;
/**
* @returns {any}
*/
  serialize(): any;
/**
*/
  readonly message: string;
}
/**
*/
export class TryingToReplaceImmutableDocumentError {
  free(): void;
/**
* @param {ExtendedDocument} extended_document
*/
  constructor(extended_document: ExtendedDocument);
}
/**
*/
export class UndefinedIndexPropertyError {
  free(): void;
/**
* @returns {string}
*/
  getDocumentType(): string;
/**
* @returns {string}
*/
  getIndexName(): string;
/**
* @returns {string}
*/
  getPropertyName(): string;
/**
* @returns {number}
*/
  getCode(): number;
/**
* @returns {any}
*/
  serialize(): any;
/**
*/
  readonly message: string;
}
/**
*/
export class UniqueIndicesLimitReachedError {
  free(): void;
/**
* @returns {string}
*/
  getDocumentType(): string;
/**
* @returns {number}
*/
  getIndexLimit(): number;
/**
* @returns {number}
*/
  getCode(): number;
/**
* @returns {any}
*/
  serialize(): any;
/**
*/
  readonly message: string;
}
/**
*/
export class UnknownAssetLockProofTypeError {
  free(): void;
/**
* @returns {number | undefined}
*/
  getType(): number | undefined;
}
/**
*/
export class UnsupportedProtocolVersionError {
  free(): void;
/**
* @returns {number}
*/
  getParsedProtocolVersion(): number;
/**
* @returns {number}
*/
  getLatestVersion(): number;
/**
* @returns {number}
*/
  getCode(): number;
/**
* @returns {any}
*/
  serialize(): any;
/**
*/
  readonly message: string;
}
/**
*/
export class ValidationResult {
  free(): void;
/**
* @param {any[] | undefined} errors_option
*/
  constructor(errors_option?: any[]);
/**
* This is just a test method - doesn't need to be in the resulted binding. Please
* remove before shipping
* @returns {(string)[]}
*/
  errorsText(): (string)[];
/**
* @returns {boolean}
*/
  isValid(): boolean;
/**
* @returns {any[]}
*/
  getErrors(): any[];
/**
* @returns {any}
*/
  getData(): any;
/**
* @returns {any}
*/
  getFirstError(): any;
/**
* @param {any} error_buffer
* @returns {any}
*/
  addError(error_buffer: any): any;
}
/**
*/
export class ValueError {
/**
** Return copy of self without private attributes.
*/
  toJSON(): Object;
/**
* Return stringified version of self.
*/
  toString(): string;
  free(): void;
/**
* @returns {string}
*/
  getMessage(): string;
/**
* @returns {number}
*/
  getCode(): number;
/**
* @returns {any}
*/
  serialize(): any;
/**
*/
  readonly message: string;
}
/**
*/
export class WrongPublicKeyPurposeError {
  free(): void;
/**
* @returns {number}
*/
  getPublicKeyPurpose(): number;
/**
* @returns {number}
*/
  getKeyPurposeRequirement(): number;
/**
* @returns {number}
*/
  getCode(): number;
/**
* @returns {any}
*/
  serialize(): any;
/**
*/
  readonly message: string;
}

export type InitInput = RequestInfo | URL | Response | BufferSource | WebAssembly.Module;

export interface InitOutput {
  readonly memory: WebAssembly.Memory;
  readonly __wbg_statetransitionfactory_free: (a: number) => void;
  readonly statetransitionfactory_new: (a: number, b: number, c: number) => void;
  readonly statetransitionfactory_createFromObject: (a: number, b: number, c: number) => number;
  readonly statetransitionfactory_createFromBuffer: (a: number, b: number, c: number, d: number) => number;
  readonly __wbg_invalidinstantassetlockproofsignatureerror_free: (a: number) => void;
  readonly invalidinstantassetlockproofsignatureerror_getCode: (a: number) => number;
  readonly invalidinstantassetlockproofsignatureerror_message: (a: number, b: number) => void;
  readonly invalidinstantassetlockproofsignatureerror_serialize: (a: number, b: number) => void;
  readonly __wbg_invalidstatetransitionsignatureerror_free: (a: number) => void;
  readonly invalidstatetransitionsignatureerror_getCode: (a: number) => number;
  readonly invalidstatetransitionsignatureerror_message: (a: number, b: number) => void;
  readonly invalidstatetransitionsignatureerror_serialize: (a: number, b: number) => void;
  readonly __wbg_identityvalidator_free: (a: number) => void;
  readonly identityvalidator_new: (a: number, b: number) => void;
  readonly identityvalidator_validate: (a: number, b: number, c: number) => void;
  readonly __wbg_signatureverificationoperation_free: (a: number) => void;
  readonly signatureverificationoperation_new: (a: number, b: number) => void;
  readonly signatureverificationoperation_getProcessingCost: (a: number, b: number) => void;
  readonly signatureverificationoperation_getStorageCost: (a: number, b: number) => void;
  readonly signatureverificationoperation_refunds: (a: number) => number;
  readonly signatureverificationoperation_toJSON: (a: number, b: number) => void;
  readonly __wbg_datacontractcreatetransition_free: (a: number) => void;
  readonly datacontractcreatetransition_new: (a: number, b: number) => void;
  readonly datacontractcreatetransition_getDataContract: (a: number) => number;
  readonly datacontractcreatetransition_getProtocolVersion: (a: number) => number;
  readonly datacontractcreatetransition_getEntropy: (a: number) => number;
  readonly datacontractcreatetransition_getOwnerId: (a: number) => number;
  readonly datacontractcreatetransition_getType: (a: number) => number;
  readonly datacontractcreatetransition_toJSON: (a: number, b: number, c: number) => void;
  readonly datacontractcreatetransition_toBuffer: (a: number, b: number, c: number) => void;
  readonly datacontractcreatetransition_getModifiedDataIds: (a: number, b: number) => void;
  readonly datacontractcreatetransition_isDataContractStateTransition: (a: number) => number;
  readonly datacontractcreatetransition_isDocumentStateTransition: (a: number) => number;
  readonly datacontractcreatetransition_isIdentityStateTransition: (a: number) => number;
  readonly datacontractcreatetransition_setExecutionContext: (a: number, b: number) => void;
  readonly datacontractcreatetransition_getExecutionContext: (a: number) => number;
  readonly datacontractcreatetransition_toObject: (a: number, b: number, c: number) => void;
  readonly datacontractcreatetransition_sign: (a: number, b: number, c: number, d: number, e: number, f: number) => void;
  readonly datacontractcreatetransition_verifySignature: (a: number, b: number, c: number, d: number) => void;
  readonly __wbg_extendeddocument_free: (a: number) => void;
  readonly extendeddocument_new: (a: number, b: number, c: number) => void;
  readonly extendeddocument_getProtocolVersion: (a: number) => number;
  readonly extendeddocument_getId: (a: number) => number;
  readonly extendeddocument_setId: (a: number, b: number) => void;
  readonly extendeddocument_getType: (a: number, b: number) => void;
  readonly extendeddocument_getDataContractId: (a: number) => number;
  readonly extendeddocument_getDataContract: (a: number) => number;
  readonly extendeddocument_setDataContractId: (a: number, b: number, c: number) => void;
  readonly extendeddocument_getDocument: (a: number) => number;
  readonly extendeddocument_setOwnerId: (a: number, b: number) => void;
  readonly extendeddocument_getOwnerId: (a: number) => number;
  readonly extendeddocument_setRevision: (a: number, b: number, c: number) => void;
  readonly extendeddocument_getRevision: (a: number, b: number) => void;
  readonly extendeddocument_setEntropy: (a: number, b: number, c: number, d: number) => void;
  readonly extendeddocument_getEntropy: (a: number) => number;
  readonly extendeddocument_setData: (a: number, b: number, c: number) => void;
  readonly extendeddocument_getData: (a: number, b: number) => void;
  readonly extendeddocument_set: (a: number, b: number, c: number, d: number, e: number) => void;
  readonly extendeddocument_get: (a: number, b: number, c: number) => number;
  readonly extendeddocument_setCreatedAt: (a: number, b: number) => void;
  readonly extendeddocument_setUpdatedAt: (a: number, b: number) => void;
  readonly extendeddocument_getCreatedAt: (a: number) => number;
  readonly extendeddocument_getUpdatedAt: (a: number) => number;
  readonly extendeddocument_getMetadata: (a: number) => number;
  readonly extendeddocument_setMetadata: (a: number, b: number, c: number) => void;
  readonly extendeddocument_toObject: (a: number, b: number, c: number) => void;
  readonly extendeddocument_toJSON: (a: number, b: number) => void;
  readonly extendeddocument_toBuffer: (a: number, b: number) => void;
  readonly extendeddocument_hash: (a: number, b: number) => void;
  readonly extendeddocument_clone: (a: number) => number;
  readonly __wbg_datatriggerexecutionresult_free: (a: number) => void;
  readonly datatriggerexecutionresult_isOk: (a: number) => number;
  readonly datatriggerexecutionresult_getErrors: (a: number) => number;
  readonly __wbg_incompatiblere2patternerror_free: (a: number) => void;
  readonly incompatiblere2patternerror_getPattern: (a: number, b: number) => void;
  readonly incompatiblere2patternerror_getPath: (a: number, b: number) => void;
  readonly incompatiblere2patternerror_getMessage: (a: number, b: number) => void;
  readonly incompatiblere2patternerror_getCode: (a: number) => number;
  readonly incompatiblere2patternerror_message: (a: number, b: number) => void;
  readonly incompatiblere2patternerror_serialize: (a: number, b: number) => void;
  readonly __wbg_invalididentitypublickeysecuritylevelerror_free: (a: number) => void;
  readonly invalididentitypublickeysecuritylevelerror_getPublicKeyId: (a: number) => number;
  readonly invalididentitypublickeysecuritylevelerror_getPublicKeyPurpose: (a: number) => number;
  readonly invalididentitypublickeysecuritylevelerror_getPublicKeySecurityLevel: (a: number) => number;
  readonly invalididentitypublickeysecuritylevelerror_getCode: (a: number) => number;
  readonly invalididentitypublickeysecuritylevelerror_message: (a: number, b: number) => void;
  readonly invalididentitypublickeysecuritylevelerror_serialize: (a: number, b: number) => void;
  readonly __wbg_datacontractalreadypresenterror_free: (a: number) => void;
  readonly datacontractalreadypresenterror_new: (a: number) => number;
  readonly datacontractalreadypresenterror_getDataContractId: (a: number) => number;
  readonly datacontractalreadypresenterror_getCode: (a: number) => number;
  readonly datacontractalreadypresenterror_message: (a: number, b: number) => void;
  readonly datacontractalreadypresenterror_serialize: (a: number, b: number) => void;
  readonly __wbg_datatriggerconditionerror_free: (a: number) => void;
  readonly datatriggerconditionerror_getDataContractId: (a: number) => number;
  readonly datatriggerconditionerror_getDocumentId: (a: number) => number;
  readonly datatriggerconditionerror_getMessage: (a: number, b: number) => void;
  readonly datatriggerconditionerror_getCode: (a: number) => number;
  readonly datatriggerconditionerror_message: (a: number, b: number) => void;
  readonly datatriggerconditionerror_serialize: (a: number, b: number) => void;
  readonly __wbg_datatriggerexecutionerror_free: (a: number) => void;
  readonly datatriggerexecutionerror_getDataContractId: (a: number) => number;
  readonly datatriggerexecutionerror_getDocumentId: (a: number) => number;
  readonly datatriggerexecutionerror_getMessage: (a: number, b: number) => void;
  readonly datatriggerexecutionerror_getCode: (a: number) => number;
  readonly datatriggerexecutionerror_message: (a: number, b: number) => void;
  readonly datatriggerexecutionerror_serialize: (a: number, b: number) => void;
  readonly __wbg_duplicateindexnameerror_free: (a: number) => void;
  readonly duplicateindexnameerror_getDocumentType: (a: number, b: number) => void;
  readonly duplicateindexnameerror_getDuplicateIndexName: (a: number, b: number) => void;
  readonly duplicateindexnameerror_getCode: (a: number) => number;
  readonly duplicateindexnameerror_message: (a: number, b: number) => void;
  readonly duplicateindexnameerror_serialize: (a: number, b: number) => void;
  readonly __wbg_invaliddocumentrevisionerror_free: (a: number) => void;
  readonly invaliddocumentrevisionerror_getDocumentId: (a: number) => number;
  readonly invaliddocumentrevisionerror_getCurrentRevision: (a: number, b: number) => void;
  readonly invaliddocumentrevisionerror_getCode: (a: number) => number;
  readonly invaliddocumentrevisionerror_message: (a: number, b: number) => void;
  readonly invaliddocumentrevisionerror_serialize: (a: number, b: number) => void;
  readonly __wbg_jsonschemavalidator_free: (a: number) => void;
  readonly jsonschemavalidator_new: (a: number, b: number, c: number) => void;
  readonly __wbg_datacontractupdatetransition_free: (a: number) => void;
  readonly datacontractupdatetransition_new: (a: number, b: number) => void;
  readonly datacontractupdatetransition_getDataContract: (a: number) => number;
  readonly datacontractupdatetransition_getProtocolVersion: (a: number) => number;
  readonly datacontractupdatetransition_getEntropy: (a: number) => number;
  readonly datacontractupdatetransition_getOwnerId: (a: number) => number;
  readonly datacontractupdatetransition_getType: (a: number) => number;
  readonly datacontractupdatetransition_toJSON: (a: number, b: number, c: number) => void;
  readonly datacontractupdatetransition_toBuffer: (a: number, b: number, c: number) => void;
  readonly datacontractupdatetransition_getModifiedDataIds: (a: number, b: number) => void;
  readonly datacontractupdatetransition_isDataContractStateTransition: (a: number) => number;
  readonly datacontractupdatetransition_isDocumentStateTransition: (a: number) => number;
  readonly datacontractupdatetransition_isIdentityStateTransition: (a: number) => number;
  readonly datacontractupdatetransition_setExecutionContext: (a: number, b: number) => void;
  readonly datacontractupdatetransition_getExecutionContext: (a: number) => number;
  readonly datacontractupdatetransition_hash: (a: number, b: number, c: number) => void;
  readonly datacontractupdatetransition_toObject: (a: number, b: number, c: number) => void;
  readonly datacontractupdatetransition_sign: (a: number, b: number, c: number, d: number, e: number, f: number) => void;
  readonly datacontractupdatetransition_verifySignature: (a: number, b: number, c: number, d: number) => void;
  readonly __wbg_datatrigger_free: (a: number) => void;
  readonly datatrigger_get_data_contract_id: (a: number) => number;
  readonly datatrigger_set_data_contract_id: (a: number, b: number, c: number) => void;
  readonly datatrigger_get_document_type: (a: number, b: number) => void;
  readonly datatrigger_set_document_type: (a: number, b: number, c: number) => void;
  readonly datatrigger_get_data_trigger_kind: (a: number, b: number) => void;
  readonly datatrigger_get_transition_action: (a: number, b: number) => void;
  readonly datatrigger_set_transition_action: (a: number, b: number, c: number, d: number) => void;
  readonly datatrigger_get_top_level_identity: (a: number) => number;
  readonly datatrigger_set_top_level_identity: (a: number, b: number, c: number) => void;
  readonly __wbg_publickeyssignaturesvalidator_free: (a: number) => void;
  readonly publickeyssignaturesvalidator_new: (a: number) => number;
  readonly publickeyssignaturesvalidator_validate: (a: number, b: number, c: number, d: number, e: number) => void;
  readonly __wbg_identitytopuptransitionstatevalidator_free: (a: number) => void;
  readonly identitytopuptransitionstatevalidator_new: (a: number) => number;
  readonly identitytopuptransitionstatevalidator_validate: (a: number, b: number) => number;
  readonly __wbg_identitypublickeydisabledatwindowviolationerror_free: (a: number) => void;
  readonly identitypublickeydisabledatwindowviolationerror_getDisabledAt: (a: number) => number;
  readonly identitypublickeydisabledatwindowviolationerror_getTimeWindowStart: (a: number) => number;
  readonly identitypublickeydisabledatwindowviolationerror_getTimeWindowEnd: (a: number) => number;
  readonly identitypublickeydisabledatwindowviolationerror_getCode: (a: number) => number;
  readonly identitypublickeydisabledatwindowviolationerror_message: (a: number, b: number) => void;
  readonly identitypublickeydisabledatwindowviolationerror_serialize: (a: number, b: number) => void;
  readonly __wbg_tryingtoreplaceimmutabledocumenterror_free: (a: number) => void;
  readonly tryingtoreplaceimmutabledocumenterror_new: (a: number) => number;
  readonly __wbg_balanceisnotenougherror_free: (a: number) => void;
  readonly balanceisnotenougherror_new: (a: number, b: number) => number;
  readonly balanceisnotenougherror_getBalance: (a: number) => number;
  readonly balanceisnotenougherror_getFee: (a: number) => number;
  readonly balanceisnotenougherror_getCode: (a: number) => number;
  readonly balanceisnotenougherror_message: (a: number, b: number) => void;
  readonly balanceisnotenougherror_serialize: (a: number, b: number) => void;
  readonly __wbg_readoperation_free: (a: number) => void;
  readonly readoperation_new: (a: number, b: number) => void;
  readonly readoperation_processingCost: (a: number, b: number) => void;
  readonly readoperation_storageCost: (a: number, b: number) => void;
  readonly readoperation_refunds: (a: number) => number;
  readonly readoperation_toJSON: (a: number, b: number) => void;
  readonly __wbg_identityupdatepublickeysvalidator_free: (a: number) => void;
  readonly identityupdatepublickeysvalidator_new: () => number;
  readonly identityupdatepublickeysvalidator_validate: (a: number, b: number, c: number, d: number) => void;
  readonly __wbg_invalidinitialrevisionerror_free: (a: number) => void;
  readonly invalidinitialrevisionerror_new: (a: number) => number;
  readonly invalidinitialrevisionerror_getDocument: (a: number) => number;
  readonly applyDocumentsBatchTransition: (a: number, b: number) => number;
  readonly validateDocumentsBatchTransitionState: (a: number, b: number) => number;
  readonly __wbg_assetlockoutputnotfounderror_free: (a: number) => void;
  readonly applyIdentityCreateTransition: (a: number, b: number) => number;
  readonly applyIdentityTopUpTransition: (a: number, b: number) => number;
  readonly applyIdentityUpdateTransition: (a: number, b: number) => number;
  readonly __wbg_documentreplacetransition_free: (a: number) => void;
  readonly documentreplacetransition_from_raw_object: (a: number, b: number, c: number) => void;
  readonly documentreplacetransition_getAction: (a: number) => number;
  readonly documentreplacetransition_getRevision: (a: number) => number;
  readonly documentreplacetransition_getUpdatedAt: (a: number) => number;
  readonly documentreplacetransition_toObject: (a: number, b: number, c: number) => void;
  readonly documentreplacetransition_toJSON: (a: number, b: number) => void;
  readonly documentreplacetransition_getData: (a: number, b: number) => void;
  readonly documentreplacetransition_getId: (a: number) => number;
  readonly documentreplacetransition_getType: (a: number, b: number) => void;
  readonly documentreplacetransition_getDataContract: (a: number) => number;
  readonly documentreplacetransition_getDataContractId: (a: number) => number;
  readonly documentreplacetransition_get: (a: number, b: number, c: number, d: number) => void;
  readonly __wbg_documenttransition_free: (a: number) => void;
  readonly documenttransition_getId: (a: number) => number;
  readonly documenttransition_getType: (a: number, b: number) => void;
  readonly documenttransition_getAction: (a: number) => number;
  readonly documenttransition_getDataContract: (a: number) => number;
  readonly documenttransition_getDataContractId: (a: number) => number;
  readonly documenttransition_setDataContractId: (a: number, b: number, c: number) => void;
  readonly documenttransition_getRevision: (a: number) => number;
  readonly documenttransition_getCreatedAt: (a: number) => number;
  readonly documenttransition_getUpdatedAt: (a: number) => number;
  readonly documenttransition_setUpdatedAt: (a: number, b: number, c: number) => void;
  readonly documenttransition_setCreatedAt: (a: number, b: number) => void;
  readonly documenttransition_getData: (a: number, b: number) => void;
  readonly documenttransition_get: (a: number, b: number, c: number) => number;
  readonly documenttransition_toObject: (a: number, b: number, c: number) => void;
  readonly documenttransition_toJSON: (a: number, b: number) => void;
  readonly documenttransition_fromTransitionCreate: (a: number) => number;
  readonly documenttransition_fromTransitionReplace: (a: number) => number;
  readonly documenttransition_fromTransitionDelete: (a: number) => number;
  readonly __wbg_datacontractgenericerror_free: (a: number) => void;
  readonly datacontractgenericerror_getMessage: (a: number, b: number) => void;
  readonly __wbg_datacontractmaxdepthexceederror_free: (a: number) => void;
  readonly datacontractmaxdeptherror_getMaxDepth: (a: number) => number;
  readonly datacontractmaxdeptherror_getSchemaDepth: (a: number) => number;
  readonly datacontractmaxdeptherror_getCode: (a: number) => number;
  readonly datacontractmaxdeptherror_message: (a: number, b: number) => void;
  readonly datacontractmaxdeptherror_serialize: (a: number, b: number) => void;
  readonly __wbg_invalidassetlockproofcorechainheighterror_free: (a: number) => void;
  readonly invalidassetlockproofcorechainheighterror_getProofCoreChainLockedHeight: (a: number) => number;
  readonly invalidassetlockproofcorechainheighterror_getCurrentCoreChainLockedHeight: (a: number) => number;
  readonly invalidassetlockproofcorechainheighterror_getCode: (a: number) => number;
  readonly invalidassetlockproofcorechainheighterror_message: (a: number, b: number) => void;
  readonly invalidassetlockproofcorechainheighterror_serialize: (a: number, b: number) => void;
  readonly __wbg_incompatibleprotocolversionerror_free: (a: number) => void;
  readonly incompatibleprotocolversionerror_getParsedProtocolVersion: (a: number) => number;
  readonly incompatibleprotocolversionerror_getMinimalProtocolVersion: (a: number) => number;
  readonly incompatibleprotocolversionerror_getCode: (a: number) => number;
  readonly incompatibleprotocolversionerror_message: (a: number, b: number) => void;
  readonly incompatibleprotocolversionerror_serialize: (a: number, b: number) => void;
  readonly __wbg_identitypublickeyisreadonlyerror_free: (a: number) => void;
  readonly identitypublickeyisreadonlyerror_getKeyId: (a: number) => number;
  readonly identitypublickeyisreadonlyerror_getPublicKeyIndex: (a: number) => number;
  readonly identitypublickeyisreadonlyerror_getCode: (a: number) => number;
  readonly identitypublickeyisreadonlyerror_message: (a: number, b: number) => void;
  readonly identitypublickeyisreadonlyerror_serialize: (a: number, b: number) => void;
  readonly __wbg_invalidinstantassetlockprooferror_free: (a: number) => void;
  readonly invalidinstantassetlockprooferror_getCode: (a: number) => number;
  readonly invalidinstantassetlockprooferror_message: (a: number, b: number) => void;
  readonly invalidinstantassetlockprooferror_serialize: (a: number, b: number) => void;
  readonly __wbg_unknownassetlockprooftypeerror_free: (a: number) => void;
  readonly unknownassetlockprooftypeerror_getType: (a: number) => number;
  readonly __wbg_statetransitionkeysignaturevalidator_free: (a: number) => void;
  readonly statetransitionkeysignaturevalidator_new: (a: number) => number;
  readonly statetransitionkeysignaturevalidator_validate: (a: number, b: number) => number;
  readonly __wbg_documentvalidator_free: (a: number) => void;
  readonly documentvalidator_new: (a: number) => number;
  readonly documentvalidator_validate: (a: number, b: number, c: number, d: number) => void;
  readonly __wbg_duplicatedidentitypublickeystateerror_free: (a: number) => void;
  readonly duplicatedidentitypublickeystateerror_getDuplicatedPublicKeysIds: (a: number) => number;
  readonly duplicatedidentitypublickeystateerror_getCode: (a: number) => number;
  readonly duplicatedidentitypublickeystateerror_message: (a: number, b: number) => void;
  readonly duplicatedidentitypublickeystateerror_serialize: (a: number, b: number) => void;
  readonly __wbg_identityalreadyexistserror_free: (a: number) => void;
  readonly identityalreadyexistserror_getIdentityId: (a: number) => number;
  readonly identityalreadyexistserror_getCode: (a: number) => number;
  readonly identityalreadyexistserror_message: (a: number, b: number) => void;
  readonly identityalreadyexistserror_serialize: (a: number, b: number) => void;
  readonly __wbg_platformvalueerror_free: (a: number) => void;
  readonly platformvalueerror_getMessage: (a: number, b: number) => void;
  readonly platformvalueerror_toString: (a: number, b: number) => void;
  readonly __wbg_invaliddatacontractversionerror_free: (a: number) => void;
  readonly invaliddatacontractversionerror_getExpectedVersion: (a: number) => number;
  readonly invaliddatacontractversionerror_getVersion: (a: number) => number;
  readonly invaliddatacontractversionerror_getCode: (a: number) => number;
  readonly invaliddatacontractversionerror_message: (a: number, b: number) => void;
  readonly invaliddatacontractversionerror_serialize: (a: number, b: number) => void;
  readonly __wbg_unsupportedprotocolversionerror_free: (a: number) => void;
  readonly unsupportedprotocolversionerror_getParsedProtocolVersion: (a: number) => number;
  readonly unsupportedprotocolversionerror_getLatestVersion: (a: number) => number;
  readonly unsupportedprotocolversionerror_getCode: (a: number) => number;
  readonly unsupportedprotocolversionerror_message: (a: number, b: number) => void;
  readonly unsupportedprotocolversionerror_serialize: (a: number, b: number) => void;
  readonly calculateStateTransitionFeeFromOperations: (a: number, b: number, c: number) => void;
  readonly __wbg_invaliddocumenttypeerror_free: (a: number) => void;
  readonly invaliddocumenttypeerror_getType: (a: number, b: number) => void;
  readonly invaliddocumenttypeerror_getDataContractId: (a: number) => number;
  readonly invaliddocumenttypeerror_getCode: (a: number) => number;
  readonly invaliddocumenttypeerror_message: (a: number, b: number) => void;
  readonly invaliddocumenttypeerror_serialize: (a: number, b: number) => void;
  readonly __wbg_identityfactory_free: (a: number) => void;
  readonly identityfactory_new: (a: number, b: number, c: number) => void;
  readonly identityfactory_create: (a: number, b: number, c: number, d: number) => void;
  readonly identityfactory_createFromObject: (a: number, b: number, c: number, d: number) => void;
  readonly identityfactory_createFromBuffer: (a: number, b: number, c: number, d: number, e: number) => void;
  readonly identityfactory_createInstantAssetLockProof: (a: number, b: number, c: number, d: number, e: number, f: number, g: number) => void;
  readonly identityfactory_createChainAssetLockProof: (a: number, b: number, c: number, d: number, e: number) => void;
  readonly identityfactory_createIdentityCreateTransition: (a: number, b: number, c: number) => void;
  readonly identityfactory_createIdentityTopUpTransition: (a: number, b: number, c: number, d: number) => void;
  readonly identityfactory_createIdentityUpdateTransition: (a: number, b: number, c: number, d: number) => void;
  readonly validateDataContractCreateTransitionState: (a: number, b: number) => number;
  readonly validateDataContractCreateTransitionBasic: (a: number) => number;
  readonly findDuplicatesByIndices: (a: number, b: number, c: number, d: number) => void;
  readonly __wbg_invalidsignaturepublickeysecuritylevelerror_free: (a: number) => void;
  readonly invalidsignaturepublickeysecuritylevelerror_getPublicKeySecurityLevel: (a: number) => number;
  readonly invalidsignaturepublickeysecuritylevelerror_getKeySecurityLevelRequirement: (a: number) => number;
  readonly invalidsignaturepublickeysecuritylevelerror_getCode: (a: number) => number;
  readonly invalidsignaturepublickeysecuritylevelerror_message: (a: number, b: number) => void;
  readonly invalidsignaturepublickeysecuritylevelerror_serialize: (a: number, b: number) => void;
  readonly __wbg_publickeysecuritylevelnotmeterror_free: (a: number) => void;
  readonly publickeysecuritylevelnotmeterror_getPublicKeySecurityLevel: (a: number) => number;
  readonly publickeysecuritylevelnotmeterror_getKeySecurityLevelRequirement: (a: number) => number;
  readonly publickeysecuritylevelnotmeterror_getCode: (a: number) => number;
  readonly publickeysecuritylevelnotmeterror_message: (a: number, b: number) => void;
  readonly publickeysecuritylevelnotmeterror_serialize: (a: number, b: number) => void;
  readonly __wbg_wrongpublickeypurposeerror_free: (a: number) => void;
  readonly wrongpublickeypurposeerror_getPublicKeyPurpose: (a: number) => number;
  readonly wrongpublickeypurposeerror_getKeyPurposeRequirement: (a: number) => number;
  readonly wrongpublickeypurposeerror_getCode: (a: number) => number;
  readonly wrongpublickeypurposeerror_message: (a: number, b: number) => void;
  readonly wrongpublickeypurposeerror_serialize: (a: number, b: number) => void;
  readonly getDataTriggers: (a: number, b: number, c: number, d: number, e: number, f: number, g: number) => void;
  readonly getAllDataTriggers: (a: number) => void;
  readonly validateDocumentsUniquenessByIndices: (a: number, b: number, c: number, d: number, e: number) => number;
  readonly __wbg_datacontractfacade_free: (a: number) => void;
  readonly datacontractfacade_create: (a: number, b: number, c: number, d: number, e: number, f: number) => void;
  readonly datacontractfacade_createFromObject: (a: number, b: number, c: number) => number;
  readonly datacontractfacade_createFromBuffer: (a: number, b: number, c: number, d: number) => number;
  readonly datacontractfacade_createDataContractCreateTransition: (a: number, b: number, c: number) => void;
  readonly datacontractfacade_createDataContractUpdateTransition: (a: number, b: number, c: number) => void;
  readonly datacontractfacade_validate: (a: number, b: number) => number;
  readonly __wbg_identityinsufficientbalanceerror_free: (a: number) => void;
  readonly identityinsufficientbalanceerror_getIdentityId: (a: number) => number;
  readonly identityinsufficientbalanceerror_getBalance: (a: number) => number;
  readonly identityinsufficientbalanceerror_getCode: (a: number) => number;
  readonly identityinsufficientbalanceerror_message: (a: number, b: number) => void;
  readonly identityinsufficientbalanceerror_serialize: (a: number, b: number) => void;
  readonly __wbg_invalididentitypublickeydataerror_free: (a: number) => void;
  readonly invalididentitypublickeydataerror_getPublicKeyId: (a: number) => number;
  readonly invalididentitypublickeydataerror_getValidationError: (a: number, b: number) => void;
  readonly invalididentitypublickeydataerror_getCode: (a: number) => number;
  readonly invalididentitypublickeydataerror_message: (a: number, b: number) => void;
  readonly invalididentitypublickeydataerror_serialize: (a: number, b: number) => void;
  readonly __wbg_invalididentityerror_free: (a: number) => void;
  readonly invalididentityerror_getErrors: (a: number, b: number) => void;
  readonly invalididentityerror_getRawIdentity: (a: number) => number;
  readonly __wbg_identitypublickeywithwitness_free: (a: number) => void;
  readonly identitypublickeywithwitness_new: (a: number, b: number) => void;
  readonly identitypublickeywithwitness_getId: (a: number) => number;
  readonly identitypublickeywithwitness_setId: (a: number, b: number) => void;
  readonly identitypublickeywithwitness_getType: (a: number) => number;
  readonly identitypublickeywithwitness_setType: (a: number, b: number, c: number) => void;
  readonly identitypublickeywithwitness_setData: (a: number, b: number, c: number, d: number) => void;
  readonly identitypublickeywithwitness_getData: (a: number) => number;
  readonly identitypublickeywithwitness_setPurpose: (a: number, b: number, c: number) => void;
  readonly identitypublickeywithwitness_getPurpose: (a: number) => number;
  readonly identitypublickeywithwitness_setSecurityLevel: (a: number, b: number, c: number) => void;
  readonly identitypublickeywithwitness_getSecurityLevel: (a: number) => number;
  readonly identitypublickeywithwitness_setReadOnly: (a: number, b: number) => void;
  readonly identitypublickeywithwitness_isReadOnly: (a: number) => number;
  readonly identitypublickeywithwitness_setSignature: (a: number, b: number, c: number) => void;
  readonly identitypublickeywithwitness_getSignature: (a: number, b: number) => void;
  readonly identitypublickeywithwitness_hash: (a: number, b: number) => void;
  readonly identitypublickeywithwitness_isMaster: (a: number) => number;
  readonly identitypublickeywithwitness_toJSON: (a: number, b: number) => void;
  readonly identitypublickeywithwitness_toObject: (a: number, b: number, c: number) => void;
  readonly __wbg_documenttransitions_free: (a: number) => void;
  readonly documenttransitions_new: () => number;
  readonly documenttransitions_addTransitionCreate: (a: number, b: number) => void;
  readonly documenttransitions_addTransitionReplace: (a: number, b: number) => void;
  readonly documenttransitions_addTransitionDelete: (a: number, b: number) => void;
  readonly __wbg_documentfactory_free: (a: number) => void;
  readonly documentfactory_new: (a: number, b: number, c: number, d: number) => number;
  readonly documentfactory_create: (a: number, b: number, c: number, d: number, e: number, f: number, g: number) => void;
  readonly documentfactory_createStateTransition: (a: number, b: number, c: number) => void;
  readonly documentfactory_createFromObject: (a: number, b: number, c: number) => number;
  readonly documentfactory_createFromBuffer: (a: number, b: number, c: number, d: number) => number;
  readonly __wbg_publickeysvalidator_free: (a: number) => void;
  readonly publickeysvalidator_new: (a: number, b: number) => void;
  readonly publickeysvalidator_validateKeys: (a: number, b: number, c: number) => void;
  readonly publickeysvalidator_validatePublicKeyStructure: (a: number, b: number, c: number) => void;
  readonly publickeysvalidator_validateKeysInStateTransition: (a: number, b: number, c: number) => void;
  readonly __wbg_instantassetlockproofstructurevalidator_free: (a: number) => void;
  readonly instantassetlockproofstructurevalidator_new: (a: number, b: number) => void;
  readonly instantassetlockproofstructurevalidator_validate: (a: number, b: number, c: number) => number;
  readonly __wbg_uniqueindiceslimitreachederror_free: (a: number) => void;
  readonly uniqueindiceslimitreachederror_getDocumentType: (a: number, b: number) => void;
  readonly uniqueindiceslimitreachederror_getIndexLimit: (a: number) => number;
  readonly uniqueindiceslimitreachederror_getCode: (a: number) => number;
  readonly uniqueindiceslimitreachederror_message: (a: number, b: number) => void;
  readonly uniqueindiceslimitreachederror_serialize: (a: number, b: number) => void;
  readonly __wbg_datacontracthavenewuniqueindexerror_free: (a: number) => void;
  readonly datacontracthavenewuniqueindexerror_getDocumentType: (a: number, b: number) => void;
  readonly datacontracthavenewuniqueindexerror_getIndexName: (a: number, b: number) => void;
  readonly datacontracthavenewuniqueindexerror_getCode: (a: number) => number;
  readonly datacontracthavenewuniqueindexerror_message: (a: number, b: number) => void;
  readonly datacontracthavenewuniqueindexerror_serialize: (a: number, b: number) => void;
  readonly __wbg_datacontractinvalidindexdefinitionupdateerror_free: (a: number) => void;
  readonly datacontractinvalidindexdefinitionupdateerror_getDocumentType: (a: number, b: number) => void;
  readonly datacontractinvalidindexdefinitionupdateerror_getIndexName: (a: number, b: number) => void;
  readonly datacontractinvalidindexdefinitionupdateerror_getCode: (a: number) => number;
  readonly datacontractinvalidindexdefinitionupdateerror_message: (a: number, b: number) => void;
  readonly datacontractinvalidindexdefinitionupdateerror_serialize: (a: number, b: number) => void;
  readonly __wbg_datacontractuniqueindiceschangederror_free: (a: number) => void;
  readonly datacontractuniqueindiceschangederror_getDocumentType: (a: number, b: number) => void;
  readonly datacontractuniqueindiceschangederror_getIndexName: (a: number, b: number) => void;
  readonly datacontractuniqueindiceschangederror_getCode: (a: number) => number;
  readonly datacontractuniqueindiceschangederror_message: (a: number, b: number) => void;
  readonly datacontractuniqueindiceschangederror_serialize: (a: number, b: number) => void;
  readonly __wbg_invalididentifiererror_free: (a: number) => void;
  readonly invalididentifiererror_getIdentifierName: (a: number, b: number) => void;
  readonly invalididentifiererror_getIdentifierError: (a: number, b: number) => void;
  readonly invalididentifiererror_getCode: (a: number) => number;
  readonly invalididentifiererror_message: (a: number, b: number) => void;
  readonly invalididentifiererror_serialize: (a: number, b: number) => void;
  readonly __wbg_incompatibledatacontractschemaerror_free: (a: number) => void;
  readonly incompatibledatacontractschemaerror_getDataContractId: (a: number) => number;
  readonly incompatibledatacontractschemaerror_getOperation: (a: number, b: number) => void;
  readonly incompatibledatacontractschemaerror_getFieldPath: (a: number, b: number) => void;
  readonly incompatibledatacontractschemaerror_getCode: (a: number) => number;
  readonly incompatibledatacontractschemaerror_message: (a: number, b: number) => void;
  readonly incompatibledatacontractschemaerror_serialize: (a: number, b: number) => void;
  readonly __wbg_publickeyvalidationerror_free: (a: number) => void;
  readonly publickeyvalidationerror_message: (a: number) => number;
  readonly __wbg_refunds_free: (a: number) => void;
  readonly refunds_identifier: (a: number) => number;
  readonly refunds_credits_per_epoch: (a: number) => number;
  readonly refunds_toObject: (a: number, b: number) => void;
  readonly __wbg_protocolversionvalidator_free: (a: number) => void;
  readonly protocolversionvalidator_new: (a: number, b: number) => void;
  readonly protocolversionvalidator_validate: (a: number, b: number, c: number) => void;
  readonly __wbg_compatibleprotocolversionisnotdefinederror_free: (a: number) => void;
  readonly compatibleprotocolversionisnotdefinederror_getCurrentProtocolVersion: (a: number) => number;
  readonly __wbg_duplicateuniqueindexerror_free: (a: number) => void;
  readonly duplicateuniqueindexerror_getDocumentId: (a: number) => number;
  readonly duplicateuniqueindexerror_getDuplicatingProperties: (a: number) => number;
  readonly duplicateuniqueindexerror_getCode: (a: number) => number;
  readonly duplicateuniqueindexerror_message: (a: number, b: number) => void;
  readonly duplicateuniqueindexerror_serialize: (a: number, b: number) => void;
  readonly __wbg_assetlockproof_free: (a: number) => void;
  readonly assetlockproof_new: (a: number, b: number) => void;
  readonly assetlockproof_createIdentifier: (a: number, b: number) => void;
  readonly assetlockproof_toObject: (a: number, b: number) => void;
  readonly createAssetLockProofInstance: (a: number, b: number) => void;
  readonly validateAssetLockTransaction: (a: number, b: number, c: number, d: number, e: number) => number;
  readonly fetchAssetLockTransactionOutput: (a: number, b: number, c: number) => number;
  readonly fetchAssetLockPublicKeyHash: (a: number, b: number, c: number) => number;
  readonly __wbg_statetransitionexecutioncontext_free: (a: number) => void;
  readonly statetransitionexecutioncontext_new: () => number;
  readonly statetransitionexecutioncontext_enableDryRun: (a: number) => void;
  readonly statetransitionexecutioncontext_disableDryRun: (a: number) => void;
  readonly statetransitionexecutioncontext_isDryRun: (a: number) => number;
  readonly statetransitionexecutioncontext_addOperation: (a: number, b: number, c: number) => void;
  readonly statetransitionexecutioncontext_getOperations: (a: number, b: number) => void;
  readonly statetransitionexecutioncontext_clearDryOperations: (a: number) => void;
  readonly generateTemporaryEcdsaPrivateKey: () => number;
  readonly __wbg_chainassetlockproof_free: (a: number) => void;
  readonly chainassetlockproof_new: (a: number, b: number) => void;
  readonly chainassetlockproof_getType: (a: number) => number;
  readonly chainassetlockproof_getCoreChainLockedHeight: (a: number) => number;
  readonly chainassetlockproof_setCoreChainLockedHeight: (a: number, b: number) => void;
  readonly chainassetlockproof_getOutPoint: (a: number) => number;
  readonly chainassetlockproof_setOutPoint: (a: number, b: number, c: number, d: number) => void;
  readonly chainassetlockproof_toJSON: (a: number, b: number) => void;
  readonly chainassetlockproof_toObject: (a: number, b: number) => void;
  readonly chainassetlockproof_createIdentifier: (a: number, b: number) => void;
  readonly __wbg_datacontractvalidator_free: (a: number) => void;
  readonly datacontractvalidator_new: () => number;
  readonly datacontractvalidator_validate: (a: number, b: number, c: number) => void;
  readonly __wbg_datacontractfactory_free: (a: number) => void;
  readonly datacontractfactory_new: (a: number, b: number, c: number) => number;
  readonly datacontractfactory_create: (a: number, b: number, c: number, d: number, e: number) => void;
  readonly datacontractfactory_createFromObject: (a: number, b: number, c: number) => number;
  readonly datacontractfactory_createFromBuffer: (a: number, b: number, c: number, d: number) => number;
  readonly datacontractfactory_createDataContractCreateTransition: (a: number, b: number) => number;
  readonly __wbg_invalidassetlockprooftransactionheighterror_free: (a: number) => void;
  readonly invalidassetlockprooftransactionheighterror_getProofCoreChainLockedHeight: (a: number) => number;
  readonly invalidassetlockprooftransactionheighterror_getTransactionHeight: (a: number, b: number) => void;
  readonly invalidassetlockprooftransactionheighterror_getCode: (a: number) => number;
  readonly invalidassetlockprooftransactionheighterror_message: (a: number, b: number) => void;
  readonly invalidassetlockprooftransactionheighterror_serialize: (a: number, b: number) => void;
  readonly __wbg_statetransitionmaxsizeexceedederror_free: (a: number) => void;
  readonly statetransitionmaxsizeexceedederror_getActualSizeKBytes: (a: number) => number;
  readonly statetransitionmaxsizeexceedederror_getMaxSizeKBytes: (a: number) => number;
  readonly statetransitionmaxsizeexceedederror_getCode: (a: number) => number;
  readonly statetransitionmaxsizeexceedederror_message: (a: number, b: number) => void;
  readonly statetransitionmaxsizeexceedederror_serialize: (a: number, b: number) => void;
  readonly __wbg_feeresult_free: (a: number) => void;
  readonly feeresult_new: () => number;
  readonly feeresult_storageFee: (a: number) => number;
  readonly feeresult_processingFee: (a: number) => number;
  readonly feeresult_feeRefunds: (a: number) => number;
  readonly feeresult_totalRefunds: (a: number) => number;
  readonly feeresult_desiredAmount: (a: number) => number;
  readonly feeresult_requiredAmount: (a: number) => number;
  readonly feeresult_set_storageFee: (a: number, b: number, c: number) => void;
  readonly feeresult_set_processingFee: (a: number, b: number, c: number) => void;
  readonly feeresult_set_feeRefunds: (a: number, b: number, c: number) => void;
  readonly feeresult_set_desiredAmount: (a: number, b: number, c: number) => void;
  readonly feeresult_set_requiredAmount: (a: number, b: number, c: number) => void;
  readonly feeresult_set_totalRefunds: (a: number, b: number, c: number) => void;
  readonly __wbg_missingpublickeyerror_free: (a: number) => void;
  readonly missingpublickeyerror_getPublicKeyId: (a: number) => number;
  readonly missingpublickeyerror_getCode: (a: number) => number;
  readonly missingpublickeyerror_message: (a: number, b: number) => void;
  readonly missingpublickeyerror_serialize: (a: number, b: number) => void;
  readonly __wbg_invalididentitypublickeyiderror_free: (a: number) => void;
  readonly invalididentitypublickeyiderror_getId: (a: number) => number;
  readonly invalididentitypublickeyiderror_getCode: (a: number) => number;
  readonly invalididentitypublickeyiderror_message: (a: number, b: number) => void;
  readonly invalididentitypublickeyiderror_serialize: (a: number, b: number) => void;
  readonly __wbg_validationresult_free: (a: number) => void;
  readonly validationresult_new: (a: number, b: number, c: number) => void;
  readonly validationresult_errorsText: (a: number, b: number) => void;
  readonly validationresult_isValid: (a: number) => number;
  readonly validationresult_getErrors: (a: number, b: number) => void;
  readonly validationresult_getData: (a: number) => number;
  readonly validationresult_getFirstError: (a: number) => number;
  readonly validationresult_addError: (a: number, b: number, c: number) => void;
  readonly __wbg_protocolversionparsingerror_free: (a: number) => void;
  readonly protocolversionparsingerror_new: (a: number, b: number) => number;
  readonly protocolversionparsingerror_getParsingError: (a: number, b: number) => void;
  readonly protocolversionparsingerror_getCode: (a: number) => number;
  readonly protocolversionparsingerror_serialize: (a: number, b: number) => void;
  readonly protocolversionparsingerror_message: (a: number, b: number) => void;
  readonly __wbg_invalidstatetransitiontypeerror_free: (a: number) => void;
  readonly invalidstatetransitiontypeerror_new: (a: number) => number;
  readonly invalidstatetransitiontypeerror_getType: (a: number) => number;
  readonly invalidstatetransitiontypeerror_getCode: (a: number) => number;
  readonly invalidstatetransitiontypeerror_message: (a: number, b: number) => void;
  readonly invalidstatetransitiontypeerror_serialize: (a: number, b: number) => void;
  readonly __wbg_nodocumentssuppliederror_free: (a: number) => void;
  readonly nodocumentssuppliederror_new: () => number;
  readonly __wbg_invalidindexedpropertyconstrainterror_free: (a: number) => void;
  readonly invalidindexedpropertyconstrainterror_getDocumentType: (a: number, b: number) => void;
  readonly invalidindexedpropertyconstrainterror_getIndexName: (a: number, b: number) => void;
  readonly invalidindexedpropertyconstrainterror_getPropertyName: (a: number, b: number) => void;
  readonly invalidindexedpropertyconstrainterror_getConstraintName: (a: number, b: number) => void;
  readonly invalidindexedpropertyconstrainterror_getReason: (a: number, b: number) => void;
  readonly invalidindexedpropertyconstrainterror_getCode: (a: number) => number;
  readonly invalidindexedpropertyconstrainterror_message: (a: number, b: number) => void;
  readonly invalidindexedpropertyconstrainterror_serialize: (a: number, b: number) => void;
  readonly executeDataTriggers: (a: number, b: number, c: number) => number;
  readonly __wbg_mismatchowneridserror_free: (a: number) => void;
  readonly mismatchowneridserror_new: (a: number, b: number) => number;
  readonly mismatchowneridserror_getDocuments: (a: number, b: number) => void;
  readonly generateDocumentId: (a: number, b: number, c: number, d: number, e: number, f: number, g: number) => void;
  readonly __wbg_dummyfeesresult_free: (a: number) => void;
  readonly dummyfeesresult_storageFee: (a: number) => number;
  readonly dummyfeesresult_processingFee: (a: number) => number;
  readonly dummyfeesresult_feeRefunds: (a: number) => number;
  readonly dummyfeesresult_set_storageFee: (a: number, b: number, c: number) => void;
  readonly dummyfeesresult_set_processingFee: (a: number, b: number, c: number) => void;
  readonly dummyfeesresult_set_feeRefunds: (a: number, b: number, c: number) => void;
  readonly decodeProtocolEntity: (a: number, b: number, c: number) => void;
  readonly __wbg_invalidjsonschemareferror_free: (a: number) => void;
  readonly invalidjsonschemareferror_getRefError: (a: number, b: number) => void;
  readonly invalidjsonschemareferror_getCode: (a: number) => number;
  readonly invalidjsonschemareferror_message: (a: number, b: number) => void;
  readonly invalidjsonschemareferror_serialize: (a: number, b: number) => void;
  readonly __wbg_invaliddocumenttransitionactionerror_free: (a: number) => void;
  readonly invaliddocumenttransitionactionerror_getAction: (a: number, b: number) => void;
  readonly invaliddocumenttransitionactionerror_getCode: (a: number) => number;
  readonly invaliddocumenttransitionactionerror_message: (a: number, b: number) => void;
  readonly invaliddocumenttransitionactionerror_serialize: (a: number, b: number) => void;
  readonly __wbg_dashplatformprotocol_free: (a: number) => void;
  readonly dashplatformprotocol_new: (a: number, b: number, c: number, d: number, e: number, f: number) => void;
  readonly dashplatformprotocol_data_contract: (a: number) => number;
  readonly dashplatformprotocol_document: (a: number) => number;
  readonly dashplatformprotocol_identity: (a: number) => number;
  readonly dashplatformprotocol_state_transition: (a: number) => number;
  readonly dashplatformprotocol_protocol_version: (a: number) => number;
  readonly dashplatformprotocol_getProtocolVersion: (a: number) => number;
  readonly dashplatformprotocol_setProtocolVersion: (a: number, b: number, c: number) => void;
  readonly dashplatformprotocol_setStateRepository: (a: number, b: number, c: number) => void;
  readonly dashplatformprotocol_getStateRepository: (a: number) => number;
  readonly fetchExtendedDocuments: (a: number, b: number, c: number) => number;
  readonly validateStateTransitionIdentitySignature: (a: number, b: number, c: number) => number;
  readonly __wbg_instantassetlockproof_free: (a: number) => void;
  readonly instantassetlockproof_new: (a: number, b: number) => void;
  readonly instantassetlockproof_getType: (a: number) => number;
  readonly instantassetlockproof_getOutputIndex: (a: number) => number;
  readonly instantassetlockproof_getOutPoint: (a: number) => number;
  readonly instantassetlockproof_getOutput: (a: number, b: number) => void;
  readonly instantassetlockproof_createIdentifier: (a: number, b: number) => void;
  readonly instantassetlockproof_getInstantLock: (a: number) => number;
  readonly instantassetlockproof_getTransaction: (a: number) => number;
  readonly instantassetlockproof_toObject: (a: number, b: number) => void;
  readonly instantassetlockproof_toJSON: (a: number, b: number) => void;
  readonly __wbg_invalididentityassetlocktransactionerror_free: (a: number) => void;
  readonly invalididentityassetlocktransactionerror_getErrorMessage: (a: number, b: number) => void;
  readonly invalididentityassetlocktransactionerror_getCode: (a: number) => number;
  readonly invalididentityassetlocktransactionerror_message: (a: number, b: number) => void;
  readonly invalididentityassetlocktransactionerror_serialize: (a: number, b: number) => void;
  readonly __wbg_jsonschemacompilationerror_free: (a: number) => void;
  readonly jsonschemacompilationerror_getError: (a: number, b: number) => void;
  readonly jsonschemacompilationerror_getCode: (a: number) => number;
  readonly jsonschemacompilationerror_message: (a: number, b: number) => void;
  readonly jsonschemacompilationerror_serialize: (a: number, b: number) => void;
  readonly __wbg_statetransitionfacade_free: (a: number) => void;
  readonly statetransitionfacade_createFromObject: (a: number, b: number, c: number) => number;
  readonly statetransitionfacade_createFromBuffer: (a: number, b: number, c: number, d: number) => number;
  readonly statetransitionfacade_validate: (a: number, b: number, c: number) => number;
  readonly statetransitionfacade_validateBasic: (a: number, b: number) => number;
  readonly statetransitionfacade_validateSignature: (a: number, b: number) => number;
  readonly statetransitionfacade_validateFee: (a: number, b: number) => number;
  readonly statetransitionfacade_validateState: (a: number, b: number) => number;
  readonly statetransitionfacade_apply: (a: number, b: number) => number;
  readonly __wbg_identitycreatetransition_free: (a: number) => void;
  readonly identitycreatetransition_new: (a: number, b: number) => void;
  readonly identitycreatetransition_setAssetLockProof: (a: number, b: number, c: number) => void;
  readonly identitycreatetransition_assetLockProof: (a: number) => number;
  readonly identitycreatetransition_getAssetLockProof: (a: number) => number;
  readonly identitycreatetransition_setPublicKeys: (a: number, b: number, c: number, d: number) => void;
  readonly identitycreatetransition_addPublicKeys: (a: number, b: number, c: number, d: number) => void;
  readonly identitycreatetransition_getPublicKeys: (a: number, b: number) => void;
  readonly identitycreatetransition_publicKeys: (a: number, b: number) => void;
  readonly identitycreatetransition_getType: (a: number) => number;
  readonly identitycreatetransition_identityId: (a: number) => number;
  readonly identitycreatetransition_getIdentityId: (a: number) => number;
  readonly identitycreatetransition_getOwnerId: (a: number) => number;
  readonly identitycreatetransition_toObject: (a: number, b: number, c: number) => void;
  readonly identitycreatetransition_toBuffer: (a: number, b: number, c: number) => void;
  readonly identitycreatetransition_toJSON: (a: number, b: number) => void;
  readonly identitycreatetransition_getModifiedDataIds: (a: number, b: number) => void;
  readonly identitycreatetransition_isDataContractStateTransition: (a: number) => number;
  readonly identitycreatetransition_isDocumentStateTransition: (a: number) => number;
  readonly identitycreatetransition_isIdentityStateTransition: (a: number) => number;
  readonly identitycreatetransition_setExecutionContext: (a: number, b: number) => void;
  readonly identitycreatetransition_getExecutionContext: (a: number) => number;
  readonly identitycreatetransition_signByPrivateKey: (a: number, b: number, c: number, d: number, e: number, f: number) => void;
  readonly identitycreatetransition_getSignature: (a: number) => number;
  readonly identitycreatetransition_setSignature: (a: number, b: number, c: number) => void;
  readonly __wbg_invalididentitycreditwithdrawaltransitionoutputscripterror_free: (a: number) => void;
  readonly invalididentitycreditwithdrawaltransitionoutputscripterror_getCode: (a: number) => number;
  readonly invalididentitycreditwithdrawaltransitionoutputscripterror_message: (a: number, b: number) => void;
  readonly invalididentitycreditwithdrawaltransitionoutputscripterror_serialize: (a: number, b: number) => void;
  readonly __wbg_missingmasterpublickeyerror_free: (a: number) => void;
  readonly missingmasterpublickeyerror_getCode: (a: number) => number;
  readonly missingmasterpublickeyerror_message: (a: number, b: number) => void;
  readonly missingmasterpublickeyerror_serialize: (a: number, b: number) => void;
  readonly __wbg_documentnorevisionerror_free: (a: number) => void;
  readonly documentnorevisionerror_new: (a: number) => number;
  readonly documentnorevisionerror_getDocument: (a: number) => number;
  readonly __wbg_serializedobjectparsingerror_free: (a: number) => void;
  readonly serializedobjectparsingerror_getParsingError: (a: number, b: number) => void;
  readonly serializedobjectparsingerror_getCode: (a: number) => number;
  readonly serializedobjectparsingerror_message: (a: number, b: number) => void;
  readonly serializedobjectparsingerror_serialize: (a: number, b: number) => void;
  readonly __wbg_identityassetlocktransactionisnotfounderror_free: (a: number) => void;
  readonly identityassetlocktransactionisnotfounderror_getTransactionId: (a: number) => number;
  readonly identityassetlocktransactionisnotfounderror_getCode: (a: number) => number;
  readonly identityassetlocktransactionisnotfounderror_message: (a: number, b: number) => void;
  readonly identityassetlocktransactionisnotfounderror_serialize: (a: number, b: number) => void;
  readonly __wbg_valueerror_free: (a: number) => void;
  readonly valueerror_getMessage: (a: number, b: number) => void;
  readonly valueerror_getCode: (a: number) => number;
  readonly valueerror_message: (a: number, b: number) => void;
  readonly valueerror_serialize: (a: number, b: number) => void;
  readonly __wbg_invalidstatetransitionerror_free: (a: number) => void;
  readonly invalidstatetransitionerror_new_wasm: (a: number, b: number, c: number, d: number) => void;
  readonly invalidstatetransitionerror_getErrors: (a: number, b: number) => void;
  readonly invalidstatetransitionerror_getRawStateTransition: (a: number) => number;
  readonly __wbg_invalidactionerror_free: (a: number) => void;
  readonly invalidactionterror_new: (a: number) => number;
  readonly __wbg_applydatacontractcreatetransition_free: (a: number) => void;
  readonly applydatacontractcreatetransition_new: (a: number) => number;
  readonly applydatacontractcreatetransition_applyDataContractCreateTransition: (a: number, b: number) => number;
  readonly __wbg_applydatacontractupdatetransition_free: (a: number) => void;
  readonly applydatacontractupdatetransition_new: (a: number) => number;
  readonly applydatacontractupdatetransition_applyDataContractUpdateTransition: (a: number, b: number) => number;
  readonly __wbg_identitycreatetransitionstatevalidator_free: (a: number) => void;
  readonly identitycreatetransitionstatevalidator_new: (a: number) => number;
  readonly identitycreatetransitionstatevalidator_validate: (a: number, b: number) => number;
  readonly __wbg_datacontractnotpresenterror_free: (a: number) => void;
  readonly datacontractnotpresenterror_getDataContractId: (a: number) => number;
  readonly datacontractnotpresenterror_getCode: (a: number) => number;
  readonly datacontractnotpresenterror_message: (a: number, b: number) => void;
  readonly datacontractnotpresenterror_serialize: (a: number, b: number) => void;
  readonly __wbg_documentalreadypresenterror_free: (a: number) => void;
  readonly documentalreadypresenterror_getDocumentId: (a: number) => number;
  readonly documentalreadypresenterror_getCode: (a: number) => number;
  readonly documentalreadypresenterror_message: (a: number, b: number) => void;
  readonly documentalreadypresenterror_serialize: (a: number, b: number) => void;
  readonly __wbg_identityfacade_free: (a: number) => void;
  readonly identityfacade_create: (a: number, b: number, c: number, d: number) => void;
  readonly identityfacade_createFromObject: (a: number, b: number, c: number, d: number) => void;
  readonly identityfacade_createFromBuffer: (a: number, b: number, c: number, d: number, e: number) => void;
  readonly identityfacade_validate: (a: number, b: number, c: number) => void;
  readonly identityfacade_createInstantAssetLockProof: (a: number, b: number, c: number, d: number, e: number, f: number, g: number) => void;
  readonly identityfacade_createChainAssetLockProof: (a: number, b: number, c: number, d: number, e: number) => void;
  readonly identityfacade_createIdentityCreateTransition: (a: number, b: number, c: number) => void;
  readonly identityfacade_createIdentityTopUpTransition: (a: number, b: number, c: number, d: number) => void;
  readonly identityfacade_createIdentityUpdateTransition: (a: number, b: number, c: number, d: number) => void;
  readonly __wbg_nonconsensuserrorwasm_free: (a: number) => void;
  readonly __wbg_inconsistentcompoundindexdataerror_free: (a: number) => void;
  readonly inconsistentcompoundindexdataerror_getIndexedProperties: (a: number) => number;
  readonly inconsistentcompoundindexdataerror_getDocumentType: (a: number, b: number) => void;
  readonly inconsistentcompoundindexdataerror_getCode: (a: number) => number;
  readonly inconsistentcompoundindexdataerror_message: (a: number, b: number) => void;
  readonly inconsistentcompoundindexdataerror_serialize: (a: number, b: number) => void;
  readonly __wbg_invaliddocumenttransitioniderror_free: (a: number) => void;
  readonly invaliddocumenttransitioniderror_getExpectedId: (a: number) => number;
  readonly invaliddocumenttransitioniderror_getInvalidId: (a: number) => number;
  readonly invaliddocumenttransitioniderror_getCode: (a: number) => number;
  readonly invaliddocumenttransitioniderror_message: (a: number, b: number) => void;
  readonly invaliddocumenttransitioniderror_serialize: (a: number, b: number) => void;
  readonly __wbg_identityassetlocktransactionoutpointalreadyexistserror_free: (a: number) => void;
  readonly identityassetlocktransactionoutpointalreadyexistserror_getOutputIndex: (a: number) => number;
  readonly identityassetlocktransactionoutpointalreadyexistserror_getTransactionId: (a: number) => number;
  readonly identityassetlocktransactionoutpointalreadyexistserror_getCode: (a: number) => number;
  readonly identityassetlocktransactionoutpointalreadyexistserror_message: (a: number, b: number) => void;
  readonly identityassetlocktransactionoutpointalreadyexistserror_serialize: (a: number, b: number) => void;
  readonly __wbg_datatriggerinvalidresulterror_free: (a: number) => void;
  readonly datatriggerinvalidresulterror_getDataContractId: (a: number) => number;
  readonly datatriggerinvalidresulterror_getDocumentId: (a: number) => number;
  readonly datatriggerinvalidresulterror_getCode: (a: number) => number;
  readonly datatriggerinvalidresulterror_message: (a: number, b: number) => void;
  readonly datatriggerinvalidresulterror_serialize: (a: number, b: number) => void;
  readonly __wbg_documentcreatetransition_free: (a: number) => void;
  readonly documentcreatetransition_from_raw_object: (a: number, b: number, c: number) => void;
  readonly documentcreatetransition_getEntropy: (a: number, b: number) => void;
  readonly documentcreatetransition_getCreatedAt: (a: number) => number;
  readonly documentcreatetransition_getUpdatedAt: (a: number) => number;
  readonly documentcreatetransition_getRevision: (a: number) => number;
  readonly documentcreatetransition_getId: (a: number) => number;
  readonly documentcreatetransition_getType: (a: number, b: number) => void;
  readonly documentcreatetransition_getAction: (a: number) => number;
  readonly documentcreatetransition_getDataContract: (a: number) => number;
  readonly documentcreatetransition_getDataContractId: (a: number) => number;
  readonly documentcreatetransition_get: (a: number, b: number, c: number, d: number) => void;
  readonly documentcreatetransition_toObject: (a: number, b: number, c: number) => void;
  readonly documentcreatetransition_toJSON: (a: number, b: number) => void;
  readonly documentcreatetransition_getData: (a: number, b: number) => void;
  readonly __wbg_datacontract_free: (a: number) => void;
  readonly datacontract_new: (a: number, b: number) => void;
  readonly datacontract_getProtocolVersion: (a: number) => number;
  readonly datacontract_getId: (a: number) => number;
  readonly datacontract_setId: (a: number, b: number, c: number) => void;
  readonly datacontract_getOwnerId: (a: number) => number;
  readonly datacontract_getVersion: (a: number) => number;
  readonly datacontract_setVersion: (a: number, b: number) => void;
  readonly datacontract_incrementVersion: (a: number) => void;
  readonly datacontract_getJsonSchemaId: (a: number, b: number) => void;
  readonly datacontract_setJsonMetaSchema: (a: number, b: number, c: number) => void;
  readonly datacontract_getJsonMetaSchema: (a: number, b: number) => void;
  readonly datacontract_setDocuments: (a: number, b: number, c: number) => void;
  readonly datacontract_getDocuments: (a: number, b: number) => void;
  readonly datacontract_isDocumentDefined: (a: number, b: number, c: number) => number;
  readonly datacontract_setDocumentSchema: (a: number, b: number, c: number, d: number, e: number) => void;
  readonly datacontract_getDocumentSchema: (a: number, b: number, c: number, d: number) => void;
  readonly datacontract_getDocumentSchemaRef: (a: number, b: number, c: number, d: number) => void;
  readonly datacontract_setDefinitions: (a: number, b: number, c: number) => void;
  readonly datacontract_getDefinitions: (a: number, b: number) => void;
  readonly datacontract_setEntropy: (a: number, b: number, c: number, d: number) => void;
  readonly datacontract_getEntropy: (a: number) => number;
  readonly datacontract_getBinaryProperties: (a: number, b: number, c: number, d: number) => void;
  readonly datacontract_getMetadata: (a: number) => number;
  readonly datacontract_setMetadata: (a: number, b: number, c: number) => void;
  readonly datacontract_toObject: (a: number, b: number) => void;
  readonly datacontract_toJSON: (a: number, b: number) => void;
  readonly datacontract_toBuffer: (a: number, b: number) => void;
  readonly datacontract_hash: (a: number, b: number) => void;
  readonly datacontract_from: (a: number, b: number) => void;
  readonly datacontract_fromBuffer: (a: number, b: number, c: number) => void;
  readonly datacontract_clone: (a: number) => number;
  readonly __wbg_datacontractdefaults_free: (a: number) => void;
  readonly datacontractdefaults_get_default_schema: (a: number) => void;
  readonly __wbg_documentsbatchtransition_free: (a: number) => void;
  readonly documentsbatchtransition_from_raw_object: (a: number, b: number, c: number) => void;
  readonly documentsbatchtransition_getType: (a: number) => number;
  readonly documentsbatchtransition_getOwnerId: (a: number) => number;
  readonly documentsbatchtransition_getTransitions: (a: number) => number;
  readonly documentsbatchtransition_setTransitions: (a: number, b: number, c: number) => void;
  readonly documentsbatchtransition_toJSON: (a: number, b: number) => void;
  readonly documentsbatchtransition_toObject: (a: number, b: number, c: number) => void;
  readonly documentsbatchtransition_getModifiedDataIds: (a: number) => number;
  readonly documentsbatchtransition_getSignaturePublicKeyId: (a: number, b: number) => void;
  readonly documentsbatchtransition_sign: (a: number, b: number, c: number, d: number, e: number, f: number) => void;
  readonly documentsbatchtransition_verifyPublicKeyLevelAndPurpose: (a: number, b: number, c: number) => void;
  readonly documentsbatchtransition_verifyPublicKeyIsEnabled: (a: number, b: number, c: number) => void;
  readonly documentsbatchtransition_verifySignature: (a: number, b: number, c: number, d: number) => void;
  readonly documentsbatchtransition_setSignaturePublicKey: (a: number, b: number) => void;
  readonly documentsbatchtransition_getKeySecurityLevelRequirement: (a: number) => number;
  readonly documentsbatchtransition_getProtocolVersion: (a: number) => number;
  readonly documentsbatchtransition_getSignature: (a: number, b: number) => void;
  readonly documentsbatchtransition_setSignature: (a: number, b: number, c: number) => void;
  readonly documentsbatchtransition_isDocumentStateTransition: (a: number) => number;
  readonly documentsbatchtransition_isDataContractStateTransition: (a: number) => number;
  readonly documentsbatchtransition_isIdentityStateTransition: (a: number) => number;
  readonly documentsbatchtransition_setExecutionContext: (a: number, b: number) => void;
  readonly documentsbatchtransition_getExecutionContext: (a: number) => number;
  readonly documentsbatchtransition_toBuffer: (a: number, b: number, c: number) => void;
  readonly documentsbatchtransition_hash: (a: number, b: number, c: number) => void;
  readonly __wbg_fetchandvalidatedatacontractfactory_free: (a: number) => void;
  readonly fetchandvalidatedatacontractfactory_new: (a: number) => number;
  readonly fetchandvalidatedatacontractfactory_validate: (a: number, b: number) => number;
  readonly fetchAndValidateDataContract: (a: number, b: number) => number;
  readonly __wbg_jsonschemaerror_free: (a: number) => void;
  readonly jsonschemaerror_getKeyword: (a: number, b: number) => void;
  readonly jsonschemaerror_getInstancePath: (a: number, b: number) => void;
  readonly jsonschemaerror_getSchemaPath: (a: number, b: number) => void;
  readonly jsonschemaerror_getPropertyName: (a: number, b: number) => void;
  readonly jsonschemaerror_getParams: (a: number, b: number) => void;
  readonly jsonschemaerror_getCode: (a: number) => number;
  readonly jsonschemaerror_toString: (a: number, b: number) => void;
  readonly jsonschemaerror_message: (a: number, b: number) => void;
  readonly jsonschemaerror_serialize: (a: number, b: number) => void;
  readonly __wbg_maxidentitypublickeylimitreachederror_free: (a: number) => void;
  readonly maxidentitypublickeylimitreachederror_getMaxItems: (a: number) => number;
  readonly maxidentitypublickeylimitreachederror_getCode: (a: number) => number;
  readonly maxidentitypublickeylimitreachederror_message: (a: number, b: number) => void;
  readonly maxidentitypublickeylimitreachederror_serialize: (a: number, b: number) => void;
  readonly findDuplicatesById: (a: number, b: number) => void;
  readonly validatePartialCompoundIndices: (a: number, b: number, c: number) => void;
  readonly validateDocumentsBatchTransitionBasic: (a: number, b: number, c: number, d: number) => number;
  readonly __wbg_datacontractnotpresentnotconsensuserror_free: (a: number) => void;
  readonly datacontractnotpresentnotconsensuserror_getDataContractId: (a: number) => number;
  readonly __wbg_identitycreatetransitionbasicvalidator_free: (a: number) => void;
  readonly identitycreatetransitionbasicvalidator_new: (a: number, b: number, c: number) => void;
  readonly identitycreatetransitionbasicvalidator_validate: (a: number, b: number, c: number) => number;
  readonly __wbg_missingdatacontractiderror_free: (a: number) => void;
  readonly missingdatacontractiderror_getCode: (a: number) => number;
  readonly missingdatacontractiderror_message: (a: number, b: number) => void;
  readonly missingdatacontractiderror_serialize: (a: number, b: number) => void;
  readonly __wbg_missingdocumenttransitionactionerror_free: (a: number) => void;
  readonly missingdocumenttransitionactionerror_getCode: (a: number) => number;
  readonly missingdocumenttransitionactionerror_message: (a: number, b: number) => void;
  readonly missingdocumenttransitionactionerror_serialize: (a: number, b: number) => void;
  readonly __wbg_missingdocumenttransitiontypeerror_free: (a: number) => void;
  readonly missingdocumenttransitiontypeerror_getCode: (a: number) => number;
  readonly missingdocumenttransitiontypeerror_message: (a: number, b: number) => void;
  readonly missingdocumenttransitiontypeerror_serialize: (a: number, b: number) => void;
  readonly __wbg_missingdocumenttypeerror_free: (a: number) => void;
  readonly missingdocumenttypeerror_getCode: (a: number) => number;
  readonly missingdocumenttypeerror_message: (a: number, b: number) => void;
  readonly missingdocumenttypeerror_serialize: (a: number, b: number) => void;
  readonly __wbg_metadata_free: (a: number) => void;
  readonly metadata_new: (a: number, b: number) => void;
  readonly metadata_from: (a: number, b: number) => void;
  readonly metadata_toJSON: (a: number) => number;
  readonly metadata_toObject: (a: number) => number;
  readonly metadata_getBlockHeight: (a: number) => number;
  readonly metadata_getCoreChainLockedHeight: (a: number) => number;
  readonly metadata_getTimeMs: (a: number) => number;
  readonly metadata_getProtocolVersion: (a: number) => number;
  readonly __wbg_identitypublickey_free: (a: number) => void;
  readonly identitypublickey_new: (a: number, b: number) => void;
  readonly identitypublickey_getId: (a: number) => number;
  readonly identitypublickey_setId: (a: number, b: number) => void;
  readonly identitypublickey_getType: (a: number) => number;
  readonly identitypublickey_setType: (a: number, b: number, c: number) => void;
  readonly identitypublickey_setData: (a: number, b: number, c: number, d: number) => void;
  readonly identitypublickey_getData: (a: number) => number;
  readonly identitypublickey_setPurpose: (a: number, b: number, c: number) => void;
  readonly identitypublickey_getPurpose: (a: number) => number;
  readonly identitypublickey_setSecurityLevel: (a: number, b: number, c: number) => void;
  readonly identitypublickey_getSecurityLevel: (a: number) => number;
  readonly identitypublickey_setReadOnly: (a: number, b: number) => void;
  readonly identitypublickey_isReadOnly: (a: number) => number;
  readonly identitypublickey_setDisabledAt: (a: number, b: number) => void;
  readonly identitypublickey_getDisabledAt: (a: number) => number;
  readonly identitypublickey_hash: (a: number, b: number) => void;
  readonly identitypublickey_isMaster: (a: number) => number;
  readonly identitypublickey_toJSON: (a: number, b: number) => void;
  readonly identitypublickey_toObject: (a: number, b: number) => void;
  readonly __wbg_revisionabsenterror_free: (a: number) => void;
  readonly revisionabsenterror_new: (a: number) => number;
  readonly revisionabsenterror_getDocument: (a: number) => number;
  readonly deserializeConsensusError: (a: number, b: number, c: number) => void;
  readonly __wbg_indexproperty_free: (a: number) => void;
  readonly indexproperty_getName: (a: number, b: number) => void;
  readonly indexproperty_isAscending: (a: number) => number;
  readonly __wbg_indexdefinition_free: (a: number) => void;
  readonly indexdefinition_getName: (a: number, b: number) => void;
  readonly indexdefinition_getProperties: (a: number, b: number) => void;
  readonly indexdefinition_isUnique: (a: number) => number;
  readonly indexdefinition_toObject: (a: number, b: number) => void;
  readonly __wbg_documenttimestampwindowviolationerror_free: (a: number) => void;
  readonly documenttimestampwindowviolationerror_getDocumentId: (a: number) => number;
  readonly documenttimestampwindowviolationerror_getTimestampName: (a: number, b: number) => void;
  readonly documenttimestampwindowviolationerror_getTimestamp: (a: number) => number;
  readonly documenttimestampwindowviolationerror_getTimeWindowStart: (a: number) => number;
  readonly documenttimestampwindowviolationerror_getTimeWindowEnd: (a: number) => number;
  readonly documenttimestampwindowviolationerror_getCode: (a: number) => number;
  readonly documenttimestampwindowviolationerror_message: (a: number, b: number) => void;
  readonly documenttimestampwindowviolationerror_serialize: (a: number, b: number) => void;
  readonly __wbg_identitytopuptransition_free: (a: number) => void;
  readonly identitytopuptransition_new: (a: number, b: number) => void;
  readonly identitytopuptransition_setAssetLockProof: (a: number, b: number, c: number) => void;
  readonly identitytopuptransition_assetLockProof: (a: number) => number;
  readonly identitytopuptransition_getAssetLockProof: (a: number) => number;
  readonly identitytopuptransition_getType: (a: number) => number;
  readonly identitytopuptransition_identityId: (a: number) => number;
  readonly identitytopuptransition_getIdentityId: (a: number) => number;
  readonly identitytopuptransition_getOwnerId: (a: number) => number;
  readonly identitytopuptransition_toObject: (a: number, b: number, c: number) => void;
  readonly identitytopuptransition_toBuffer: (a: number, b: number, c: number) => void;
  readonly identitytopuptransition_toJSON: (a: number, b: number) => void;
  readonly identitytopuptransition_getModifiedDataIds: (a: number, b: number) => void;
  readonly identitytopuptransition_isDataContractStateTransition: (a: number) => number;
  readonly identitytopuptransition_isDocumentStateTransition: (a: number) => number;
  readonly identitytopuptransition_isIdentityStateTransition: (a: number) => number;
  readonly identitytopuptransition_setExecutionContext: (a: number, b: number) => void;
  readonly identitytopuptransition_getExecutionContext: (a: number) => number;
  readonly identitytopuptransition_signByPrivateKey: (a: number, b: number, c: number, d: number, e: number, f: number) => void;
  readonly identitytopuptransition_getSignature: (a: number) => number;
  readonly identitytopuptransition_setSignature: (a: number, b: number, c: number) => void;
  readonly __wbg_duplicatedocumenttransitionswithidserror_free: (a: number) => void;
  readonly duplicatedocumenttransitionswithidserror_getDocumentTransitionReferences: (a: number) => number;
  readonly duplicatedocumenttransitionswithidserror_getCode: (a: number) => number;
  readonly duplicatedocumenttransitionswithidserror_message: (a: number, b: number) => void;
  readonly duplicatedocumenttransitionswithidserror_serialize: (a: number, b: number) => void;
  readonly __wbg_identityupdatetransitionbasicvalidator_free: (a: number) => void;
  readonly identityupdatetransitionbasicvalidator_new: (a: number, b: number) => void;
  readonly identityupdatetransitionbasicvalidator_validate: (a: number, b: number, c: number) => void;
  readonly __wbg_identityassetlocktransactionoutputnotfounderror_free: (a: number) => void;
  readonly identityassetlocktransactionoutputnotfounderror_getOutputIndex: (a: number) => number;
  readonly identityassetlocktransactionoutputnotfounderror_getCode: (a: number) => number;
  readonly identityassetlocktransactionoutputnotfounderror_message: (a: number, b: number) => void;
  readonly identityassetlocktransactionoutputnotfounderror_serialize: (a: number, b: number) => void;
  readonly __wbg_invalidassetlocktransactionoutputreturnsizeerror_free: (a: number) => void;
  readonly invalidassetlocktransactionoutputreturnsizeerror_getOutputIndex: (a: number) => number;
  readonly invalidassetlocktransactionoutputreturnsizeerror_getCode: (a: number) => number;
  readonly invalidassetlocktransactionoutputreturnsizeerror_message: (a: number, b: number) => void;
  readonly invalidassetlocktransactionoutputreturnsizeerror_serialize: (a: number, b: number) => void;
  readonly __wbg_invalididentitykeysignatureerror_free: (a: number) => void;
  readonly invalididentitykeysignatureerror_getPublicKeyId: (a: number) => number;
  readonly invalididentitykeysignatureerror_getCode: (a: number) => number;
  readonly invalididentitykeysignatureerror_message: (a: number, b: number) => void;
  readonly invalididentitykeysignatureerror_serialize: (a: number, b: number) => void;
  readonly __wbg_publickeyisdisablederror_free: (a: number) => void;
  readonly publickeyisdisablederror_getPublicKeyId: (a: number) => number;
  readonly publickeyisdisablederror_getCode: (a: number) => number;
  readonly publickeyisdisablederror_message: (a: number, b: number) => void;
  readonly publickeyisdisablederror_serialize: (a: number, b: number) => void;
  readonly validateDataContractUpdateTransitionState: (a: number, b: number) => number;
  readonly validateIndicesAreBackwardCompatible: (a: number, b: number, c: number) => void;
  readonly validateDataContractUpdateTransitionBasic: (a: number, b: number, c: number) => number;
  readonly __wbg_invaliddocumenterror_free: (a: number) => void;
  readonly invaliddocumenterror_new: (a: number, b: number, c: number) => number;
  readonly invaliddocumenterror_getErrors: (a: number, b: number) => void;
  readonly invaliddocumenterror_getRawDocument: (a: number) => number;
  readonly __wbg_assetlocktransactionisnotfounderror_free: (a: number) => void;
  readonly assetlocktransactionisnotfounderror_getTransactionId: (a: number) => number;
  readonly __wbg_identityupdatetransitionstatevalidator_free: (a: number) => void;
  readonly identityupdatetransitionstatevalidator_new: (a: number, b: number) => void;
  readonly identityupdatetransitionstatevalidator_validate: (a: number, b: number) => number;
  readonly __wbg_invalidactionnameerror_free: (a: number) => void;
  readonly invalidactionnameerror_new: (a: number, b: number) => number;
  readonly invalidactionnameerror_getActions: (a: number, b: number) => void;
  readonly __wbg_documentowneridmismatcherror_free: (a: number) => void;
  readonly documentowneridmismatcherror_getDocumentId: (a: number) => number;
  readonly documentowneridmismatcherror_getDocumentOwnerId: (a: number) => number;
  readonly documentowneridmismatcherror_getExistingDocumentOwnerId: (a: number) => number;
  readonly documentowneridmismatcherror_getCode: (a: number) => number;
  readonly documentowneridmismatcherror_message: (a: number, b: number) => void;
  readonly documentowneridmismatcherror_serialize: (a: number, b: number) => void;
  readonly calculateStateTransitionFee: (a: number, b: number) => void;
  readonly __wbg_identityupdatetransition_free: (a: number) => void;
  readonly identityupdatetransition_new: (a: number, b: number) => void;
  readonly identityupdatetransition_setPublicKeysToAdd: (a: number, b: number, c: number, d: number) => void;
  readonly identityupdatetransition_getPublicKeysToAdd: (a: number, b: number) => void;
  readonly identityupdatetransition_addPublicKeys: (a: number, b: number) => void;
  readonly identityupdatetransition_getPublicKeyIdsToDisable: (a: number, b: number) => void;
  readonly identityupdatetransition_setPublicKeyIdsToDisable: (a: number, b: number, c: number) => void;
  readonly identityupdatetransition_getPublicKeysDisabledAt: (a: number) => number;
  readonly identityupdatetransition_setPublicKeysDisabledAt: (a: number, b: number) => void;
  readonly identityupdatetransition_getType: (a: number) => number;
  readonly identityupdatetransition_identityId: (a: number) => number;
  readonly identityupdatetransition_getIdentityId: (a: number) => number;
  readonly identityupdatetransition_setIdentityId: (a: number, b: number) => void;
  readonly identityupdatetransition_getOwnerId: (a: number) => number;
  readonly identityupdatetransition_toObject: (a: number, b: number, c: number) => void;
  readonly identityupdatetransition_toBuffer: (a: number, b: number, c: number) => void;
  readonly identityupdatetransition_toJSON: (a: number, b: number) => void;
  readonly identityupdatetransition_getModifiedDataIds: (a: number, b: number) => void;
  readonly identityupdatetransition_isDataContractStateTransition: (a: number) => number;
  readonly identityupdatetransition_isDocumentStateTransition: (a: number) => number;
  readonly identityupdatetransition_isIdentityStateTransition: (a: number) => number;
  readonly identityupdatetransition_setExecutionContext: (a: number, b: number) => void;
  readonly identityupdatetransition_getExecutionContext: (a: number) => number;
  readonly identityupdatetransition_signByPrivateKey: (a: number, b: number, c: number, d: number, e: number, f: number) => void;
  readonly identityupdatetransition_setSignaturePublicKeyId: (a: number, b: number, c: number) => void;
  readonly identityupdatetransition_getSignature: (a: number) => number;
  readonly identityupdatetransition_setSignature: (a: number, b: number, c: number) => void;
  readonly identityupdatetransition_getRevision: (a: number) => number;
  readonly identityupdatetransition_setRevision: (a: number, b: number) => void;
  readonly identityupdatetransition_sign: (a: number, b: number, c: number, d: number, e: number, f: number) => void;
  readonly identityupdatetransition_verifySignature: (a: number, b: number, c: number, d: number) => void;
  readonly __wbg_document_free: (a: number) => void;
  readonly document_new: (a: number, b: number, c: number, d: number) => void;
  readonly document_getId: (a: number) => number;
  readonly document_setId: (a: number, b: number) => void;
  readonly document_setOwnerId: (a: number, b: number) => void;
  readonly document_getOwnerId: (a: number) => number;
  readonly document_setRevision: (a: number, b: number, c: number) => void;
  readonly document_getRevision: (a: number, b: number) => void;
  readonly document_setData: (a: number, b: number, c: number) => void;
  readonly document_getData: (a: number, b: number) => void;
  readonly document_set: (a: number, b: number, c: number, d: number, e: number) => void;
  readonly document_get: (a: number, b: number, c: number, d: number) => void;
  readonly document_setCreatedAt: (a: number, b: number) => void;
  readonly document_setUpdatedAt: (a: number, b: number) => void;
  readonly document_getCreatedAt: (a: number) => number;
  readonly document_getUpdatedAt: (a: number) => number;
  readonly document_toObject: (a: number, b: number, c: number, d: number, e: number, f: number) => void;
  readonly document_toJSON: (a: number, b: number) => void;
  readonly document_toBuffer: (a: number, b: number) => void;
  readonly document_hash: (a: number, b: number, c: number, d: number, e: number) => void;
  readonly document_clone: (a: number) => number;
  readonly __wbg_duplicatedocumenttransitionswithindiceserror_free: (a: number) => void;
  readonly duplicatedocumenttransitionswithindiceserror_getDocumentTransitionReferences: (a: number) => number;
  readonly duplicatedocumenttransitionswithindiceserror_getCode: (a: number) => number;
  readonly duplicatedocumenttransitionswithindiceserror_message: (a: number, b: number) => void;
  readonly duplicatedocumenttransitionswithindiceserror_serialize: (a: number, b: number) => void;
  readonly __wbg_duplicatedidentitypublickeyerror_free: (a: number) => void;
  readonly duplicatedidentitypublickeyerror_getDuplicatedPublicKeysIds: (a: number) => number;
  readonly duplicatedidentitypublickeyerror_getCode: (a: number) => number;
  readonly duplicatedidentitypublickeyerror_message: (a: number, b: number) => void;
  readonly duplicatedidentitypublickeyerror_serialize: (a: number, b: number) => void;
  readonly __wbg_duplicatedidentitypublickeyiderror_free: (a: number) => void;
  readonly duplicatedidentitypublickeyiderror_getDuplicatedIds: (a: number) => number;
  readonly duplicatedidentitypublickeyiderror_getCode: (a: number) => number;
  readonly duplicatedidentitypublickeyiderror_message: (a: number, b: number) => void;
  readonly duplicatedidentitypublickeyiderror_serialize: (a: number, b: number) => void;
  readonly __wbg_duplicatedidentitypublickeyidstateerror_free: (a: number) => void;
  readonly duplicatedidentitypublickeyidstateerror_getDuplicatedIds: (a: number) => number;
  readonly duplicatedidentitypublickeyidstateerror_getCode: (a: number) => number;
  readonly duplicatedidentitypublickeyidstateerror_message: (a: number, b: number) => void;
  readonly duplicatedidentitypublickeyidstateerror_serialize: (a: number, b: number) => void;
  readonly __wbg_identitynotfounderror_free: (a: number) => void;
  readonly identitynotfounderror_new: (a: number) => number;
  readonly identitynotfounderror_getIdentityId: (a: number) => number;
  readonly identitynotfounderror_getCode: (a: number) => number;
  readonly identitynotfounderror_message: (a: number, b: number) => void;
  readonly identitynotfounderror_serialize: (a: number, b: number) => void;
  readonly __wbg_identitytopuptransitionbasicvalidator_free: (a: number) => void;
  readonly identitytopuptransitionbasicvalidator_new: (a: number, b: number) => void;
  readonly identitytopuptransitionbasicvalidator_validate: (a: number, b: number, c: number) => number;
  readonly __wbg_missingstatetransitiontypeerror_free: (a: number) => void;
  readonly missingstatetransitiontypeerror_new: () => number;
  readonly missingstatetransitiontypeerror_getCode: (a: number) => number;
  readonly missingstatetransitiontypeerror_message: (a: number, b: number) => void;
  readonly missingstatetransitiontypeerror_serialize: (a: number, b: number) => void;
  readonly __wbg_systempropertyindexalreadypresenterror_free: (a: number) => void;
  readonly systempropertyindexalreadypresenterror_getDocumentType: (a: number, b: number) => void;
  readonly systempropertyindexalreadypresenterror_getIndexName: (a: number, b: number) => void;
  readonly systempropertyindexalreadypresenterror_getPropertyName: (a: number, b: number) => void;
  readonly systempropertyindexalreadypresenterror_getCode: (a: number) => number;
  readonly systempropertyindexalreadypresenterror_message: (a: number, b: number) => void;
  readonly systempropertyindexalreadypresenterror_serialize: (a: number, b: number) => void;
  readonly __wbg_undefinedindexpropertyerror_free: (a: number) => void;
  readonly undefinedindexpropertyerror_getDocumentType: (a: number, b: number) => void;
  readonly undefinedindexpropertyerror_getIndexName: (a: number, b: number) => void;
  readonly undefinedindexpropertyerror_getPropertyName: (a: number, b: number) => void;
  readonly undefinedindexpropertyerror_getCode: (a: number) => number;
  readonly undefinedindexpropertyerror_message: (a: number, b: number) => void;
  readonly undefinedindexpropertyerror_serialize: (a: number, b: number) => void;
  readonly __wbg_documentnotfounderror_free: (a: number) => void;
  readonly documentnotfounderror_getDocumentId: (a: number) => number;
  readonly documentnotfounderror_getCode: (a: number) => number;
  readonly documentnotfounderror_message: (a: number, b: number) => void;
  readonly documentnotfounderror_serialize: (a: number, b: number) => void;
  readonly __wbg_documenttimestampsmismatcherror_free: (a: number) => void;
  readonly documenttimestampsmismatcherror_getDocumentId: (a: number) => number;
  readonly documenttimestampsmismatcherror_getCode: (a: number) => number;
  readonly documenttimestampsmismatcherror_message: (a: number, b: number) => void;
  readonly documenttimestampsmismatcherror_serialize: (a: number, b: number) => void;
  readonly __wbg_operation_free: (a: number) => void;
  readonly __wbg_precalculatedoperation_free: (a: number) => void;
  readonly precalculatedoperation_new: (a: number, b: number, c: number, d: number) => void;
  readonly precalculatedoperation_fromFee: (a: number) => number;
  readonly precalculatedoperation_getProcessingCost: (a: number, b: number) => void;
  readonly precalculatedoperation_getStorageCost: (a: number, b: number) => void;
  readonly precalculatedoperation_refunds: (a: number) => number;
  readonly precalculatedoperation_refunds_as_objects: (a: number, b: number) => void;
  readonly precalculatedoperation_toJSON: (a: number, b: number) => void;
  readonly __wbg_invaliddocumenttypeindatacontracterror_free: (a: number) => void;
  readonly invaliddocumenttypeindatacontracterror_new: (a: number, b: number, c: number) => number;
  readonly invaliddocumenttypeindatacontracterror_getType: (a: number, b: number) => void;
  readonly invaliddocumenttypeindatacontracterror_getDataContractId: (a: number) => number;
  readonly __wbg_datacontractimmutablepropertiesupdateerror_free: (a: number) => void;
  readonly datacontractimmutablepropertiesupdateerror_getOperation: (a: number, b: number) => void;
  readonly datacontractimmutablepropertiesupdateerror_getFieldPath: (a: number, b: number) => void;
  readonly datacontractimmutablepropertiesupdateerror_getCode: (a: number) => number;
  readonly datacontractimmutablepropertiesupdateerror_message: (a: number, b: number) => void;
  readonly datacontractimmutablepropertiesupdateerror_serialize: (a: number, b: number) => void;
  readonly __wbg_duplicateindexerror_free: (a: number) => void;
  readonly duplicateindexerror_getDocumentType: (a: number, b: number) => void;
  readonly duplicateindexerror_getIndexName: (a: number, b: number) => void;
  readonly duplicateindexerror_getCode: (a: number) => number;
  readonly duplicateindexerror_message: (a: number, b: number) => void;
  readonly duplicateindexerror_serialize: (a: number, b: number) => void;
  readonly __wbg_invalidcompoundindexerror_free: (a: number) => void;
  readonly invalidcompoundindexerror_getDocumentType: (a: number, b: number) => void;
  readonly invalidcompoundindexerror_getIndexName: (a: number, b: number) => void;
  readonly invalidcompoundindexerror_getCode: (a: number) => number;
  readonly invalidcompoundindexerror_message: (a: number, b: number) => void;
  readonly invalidcompoundindexerror_serialize: (a: number, b: number) => void;
  readonly __wbg_invaliddatacontracterror_free: (a: number) => void;
  readonly invaliddatacontracterror_getErrors: (a: number, b: number) => void;
  readonly invaliddatacontracterror_getRawDataContract: (a: number) => number;
  readonly invaliddatacontracterror_getMessage: (a: number, b: number) => void;
  readonly __wbg_invalididentityrevisionerror_free: (a: number) => void;
  readonly invalididentityrevisionerror_getIdentityId: (a: number) => number;
  readonly invalididentityrevisionerror_getCurrentRevision: (a: number) => number;
  readonly invalididentityrevisionerror_getCode: (a: number) => number;
  readonly invalididentityrevisionerror_message: (a: number, b: number) => void;
  readonly invalididentityrevisionerror_serialize: (a: number, b: number) => void;
  readonly __wbg_documentfacade_free: (a: number) => void;
  readonly documentfacade_new: (a: number, b: number, c: number) => number;
  readonly documentfacade_create: (a: number, b: number, c: number, d: number, e: number, f: number, g: number) => void;
  readonly documentfacade_createFromObject: (a: number, b: number, c: number) => number;
  readonly documentfacade_createFromBuffer: (a: number, b: number, c: number, d: number) => number;
  readonly documentfacade_createStateTransition: (a: number, b: number, c: number) => void;
  readonly documentfacade_validate: (a: number, b: number) => number;
  readonly documentfacade_validate_raw_document: (a: number, b: number) => number;
  readonly __wbg_invalididentityassetlocktransactionoutputerror_free: (a: number) => void;
  readonly invalididentityassetlocktransactionoutputerror_getOutputIndex: (a: number) => number;
  readonly invalididentityassetlocktransactionoutputerror_getCode: (a: number) => number;
  readonly invalididentityassetlocktransactionoutputerror_message: (a: number, b: number) => void;
  readonly invalididentityassetlocktransactionoutputerror_serialize: (a: number, b: number) => void;
  readonly __wbg_invalididentitycreditwithdrawaltransitioncorefeeerror_free: (a: number) => void;
  readonly invalididentitycreditwithdrawaltransitioncorefeeerror_getCoreFee: (a: number) => number;
  readonly invalididentitycreditwithdrawaltransitioncorefeeerror_getCode: (a: number) => number;
  readonly invalididentitycreditwithdrawaltransitioncorefeeerror_message: (a: number, b: number) => void;
  readonly invalididentitycreditwithdrawaltransitioncorefeeerror_serialize: (a: number, b: number) => void;
  readonly __wbg_notimplementedidentitycreditwithdrawaltransitionpoolingerror_free: (a: number) => void;
  readonly notimplementedidentitycreditwithdrawaltransitionpoolingerror_getPooling: (a: number) => number;
  readonly notimplementedidentitycreditwithdrawaltransitionpoolingerror_getCode: (a: number) => number;
  readonly notimplementedidentitycreditwithdrawaltransitionpoolingerror_message: (a: number, b: number) => void;
  readonly notimplementedidentitycreditwithdrawaltransitionpoolingerror_serialize: (a: number, b: number) => void;
  readonly __wbg_identitypublickeyisdisablederror_free: (a: number) => void;
  readonly identitypublickeyisdisablederror_getPublicKeyIndex: (a: number) => number;
  readonly identitypublickeyisdisablederror_getCode: (a: number) => number;
  readonly identitypublickeyisdisablederror_message: (a: number, b: number) => void;
  readonly identitypublickeyisdisablederror_serialize: (a: number, b: number) => void;
  readonly __wbg_datatriggerexecutioncontext_free: (a: number) => void;
  readonly datatriggerexecutioncontext_new: (a: number, b: number, c: number, d: number, e: number) => void;
  readonly datatriggerexecutioncontext_ownerId: (a: number) => number;
  readonly datatriggerexecutioncontext_set_ownerId: (a: number, b: number, c: number) => void;
  readonly datatriggerexecutioncontext_dataContract: (a: number) => number;
  readonly datatriggerexecutioncontext_set_dataContract: (a: number, b: number) => void;
  readonly datatriggerexecutioncontext_stateTransitionExecutionContext: (a: number) => number;
  readonly datatriggerexecutioncontext_set_statTransitionExecutionContext: (a: number, b: number) => void;
  readonly __wbg_documentdeletetransition_free: (a: number) => void;
  readonly documentdeletetransition_getAction: (a: number) => number;
  readonly documentdeletetransition_toObject: (a: number, b: number, c: number) => void;
  readonly documentdeletetransition_toJSON: (a: number, b: number) => void;
  readonly documentdeletetransition_getId: (a: number) => number;
  readonly documentdeletetransition_getType: (a: number, b: number) => void;
  readonly documentdeletetransition_getDataContract: (a: number) => number;
  readonly documentdeletetransition_getDataContractId: (a: number) => number;
  readonly documentdeletetransition_get: (a: number, b: number, c: number, d: number) => void;
  readonly __wbg_invaliddatacontractiderror_free: (a: number) => void;
  readonly invaliddatacontractiderror_getExpectedId: (a: number) => number;
  readonly invaliddatacontractiderror_getInvalidId: (a: number) => number;
  readonly invaliddatacontractiderror_getCode: (a: number) => number;
  readonly invaliddatacontractiderror_message: (a: number, b: number) => void;
  readonly invaliddatacontractiderror_serialize: (a: number, b: number) => void;
  readonly __wbg_identityassetlockprooflockedtransactionmismatcherror_free: (a: number) => void;
  readonly identityassetlockprooflockedtransactionmismatcherror_getInstantLockTransactionId: (a: number) => number;
  readonly identityassetlockprooflockedtransactionmismatcherror_getAssetLockTransactionId: (a: number) => number;
  readonly identityassetlockprooflockedtransactionmismatcherror_getCode: (a: number) => number;
  readonly identityassetlockprooflockedtransactionmismatcherror_message: (a: number, b: number) => void;
  readonly identityassetlockprooflockedtransactionmismatcherror_serialize: (a: number, b: number) => void;
  readonly calculateOperationFees: (a: number, b: number) => void;
  readonly __wbg_chainassetlockproofstructurevalidator_free: (a: number) => void;
  readonly chainassetlockproofstructurevalidator_new: (a: number, b: number) => void;
  readonly chainassetlockproofstructurevalidator_validate: (a: number, b: number, c: number) => number;
  readonly __wbg_invalidindexpropertytypeerror_free: (a: number) => void;
  readonly invalidindexpropertytypeerror_getDocumentType: (a: number, b: number) => void;
  readonly invalidindexpropertytypeerror_getIndexName: (a: number, b: number) => void;
  readonly invalidindexpropertytypeerror_getPropertyName: (a: number, b: number) => void;
  readonly invalidindexpropertytypeerror_getPropertyType: (a: number, b: number) => void;
  readonly invalidindexpropertytypeerror_getCode: (a: number) => number;
  readonly invalidindexpropertytypeerror_message: (a: number, b: number) => void;
  readonly invalidindexpropertytypeerror_serialize: (a: number, b: number) => void;
  readonly __wbg_invalididentitypublickeytypeerror_free: (a: number) => void;
  readonly invalididentitypublickeytypeerror_new: (a: number, b: number) => void;
  readonly invalididentitypublickeytypeerror_getPublicKeyType: (a: number) => number;
  readonly invalididentitypublickeytypeerror_getCode: (a: number) => number;
  readonly invalididentitypublickeytypeerror_message: (a: number, b: number) => void;
  readonly invalididentitypublickeytypeerror_serialize: (a: number, b: number) => void;
  readonly __wbg_documentalreadyexistserror_free: (a: number) => void;
  readonly documentalreadyexistserror_getDocumentTransition: (a: number) => number;
  readonly __wbg_documentnotprovidederror_free: (a: number) => void;
  readonly documentnotprovidederror_getDocumentTransition: (a: number) => number;
  readonly __wbg_invaliddocumentactionerror_free: (a: number) => void;
  readonly invaliddocumentactionerror_getDocumentTransition: (a: number) => number;
  readonly __wbg_identity_free: (a: number) => void;
  readonly identity_new: (a: number, b: number) => void;
  readonly identity_getProtocolVersion: (a: number) => number;
  readonly identity_getId: (a: number) => number;
  readonly identity_setPublicKeys: (a: number, b: number, c: number) => void;
  readonly identity_getPublicKeys: (a: number, b: number) => void;
  readonly identity_getPublicKeyById: (a: number, b: number) => number;
  readonly identity_getBalance: (a: number) => number;
  readonly identity_setBalance: (a: number, b: number) => void;
  readonly identity_increaseBalance: (a: number, b: number) => number;
  readonly identity_reduceBalance: (a: number, b: number) => number;
  readonly identity_setAssetLockProof: (a: number, b: number, c: number) => void;
  readonly identity_getAssetLockProof: (a: number) => number;
  readonly identity_setRevision: (a: number, b: number) => void;
  readonly identity_getRevision: (a: number) => number;
  readonly identity_setMetadata: (a: number, b: number, c: number) => void;
  readonly identity_getMetadata: (a: number) => number;
  readonly identity_from: (a: number) => number;
  readonly identity_toJSON: (a: number, b: number) => void;
  readonly identity_toObject: (a: number, b: number) => void;
  readonly identity_toBuffer: (a: number, b: number) => void;
  readonly identity_hash: (a: number, b: number) => void;
  readonly identity_addPublicKey: (a: number, b: number) => void;
  readonly identity_addPublicKeys: (a: number, b: number, c: number) => void;
  readonly identity_getPublicKeyMaxId: (a: number) => number;
  readonly rustsecp256k1_v0_6_1_context_create: (a: number) => number;
  readonly rustsecp256k1_v0_6_1_context_destroy: (a: number) => void;
  readonly rustsecp256k1_v0_6_1_default_illegal_callback_fn: (a: number, b: number) => void;
  readonly rustsecp256k1_v0_6_1_default_error_callback_fn: (a: number, b: number) => void;
  readonly __wbindgen_malloc: (a: number) => number;
  readonly __wbindgen_realloc: (a: number, b: number, c: number) => number;
  readonly __wbindgen_export_2: WebAssembly.Table;
  readonly _dyn_core__ops__function__FnMut__A____Output___R_as_wasm_bindgen__closure__WasmClosure___describe__invoke__hdbb4810ee474a6bf: (a: number, b: number, c: number) => void;
  readonly __wbindgen_add_to_stack_pointer: (a: number) => number;
  readonly __wbindgen_free: (a: number, b: number) => void;
  readonly __wbindgen_exn_store: (a: number) => void;
  readonly wasm_bindgen__convert__closures__invoke2_mut__hf8604d23cbf0f65f: (a: number, b: number, c: number, d: number) => void;
}

export type SyncInitInput = BufferSource | WebAssembly.Module;
/**
* Instantiates the given `module`, which can either be bytes or
* a precompiled `WebAssembly.Module`.
*
* @param {SyncInitInput} module
*
* @returns {InitOutput}
*/
export function initSync(module: SyncInitInput): InitOutput;

/**
* If `module_or_path` is {RequestInfo} or {URL}, makes a request and
* for everything else, calls `WebAssembly.instantiate` directly.
*
* @param {InitInput | Promise<InitInput>} module_or_path
*
* @returns {Promise<InitOutput>}
*/
export default function init (module_or_path: InitInput | Promise<InitInput>): Promise<InitOutput>;
