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
export const KeyType: Readonly<{
    ECDSA_SECP256K1: number;
    0: string;
    BLS12_381: number;
    1: string;
    ECDSA_HASH160: number;
    2: string;
    BIP13_SCRIPT_HASH: number;
    3: string;
}>;
/**
*/
export const KeySecurityLevel: Readonly<{
    MASTER: number;
    0: string;
    CRITICAL: number;
    1: string;
    HIGH: number;
    2: string;
    MEDIUM: number;
    3: string;
}>;
/**
*/
export const KeyPurpose: Readonly<{
    /**
    * at least one authentication key must be registered for all security levels
    */
    AUTHENTICATION: number;
    0: string;
    /**
    * this key cannot be used for signing documents
    */
    ENCRYPTION: number;
    1: string;
    /**
    * this key cannot be used for signing documents
    */
    DECRYPTION: number;
    2: string;
    WITHDRAW: number;
    3: string;
}>;
/**
*/
export class ApplyDataContractCreateTransition {
    static __wrap(ptr: any): any;
    /**
    * @param {any} state_repository
    */
    constructor(state_repository: any);
    __destroy_into_raw(): number;
    ptr: number;
    free(): void;
    /**
    * @param {DataContractCreateTransition} transition
    * @returns {Promise<void>}
    */
    applyDataContractCreateTransition(transition: DataContractCreateTransition): Promise<void>;
}
/**
*/
export class ApplyDataContractUpdateTransition {
    static __wrap(ptr: any): any;
    /**
    * @param {any} state_repository
    */
    constructor(state_repository: any);
    __destroy_into_raw(): number;
    ptr: number;
    free(): void;
    /**
    * @param {DataContractUpdateTransition} transition
    * @returns {Promise<void>}
    */
    applyDataContractUpdateTransition(transition: DataContractUpdateTransition): Promise<void>;
}
/**
*/
export class AssetLockOutputNotFoundError {
    static __wrap(ptr: any): any;
    __destroy_into_raw(): number;
    ptr: number;
    free(): void;
}
/**
*/
export class AssetLockProof {
    static __wrap(ptr: any): any;
    /**
    * @param {any} raw_asset_lock_proof
    */
    constructor(raw_asset_lock_proof: any);
    __destroy_into_raw(): number;
    ptr: number;
    free(): void;
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
    static __wrap(ptr: any): any;
    __destroy_into_raw(): number;
    ptr: number;
    free(): void;
    /**
    * @returns {any}
    */
    getTransactionId(): any;
}
/**
*/
export class BalanceIsNotEnoughError {
    static __wrap(ptr: any): any;
    /**
    * @param {bigint} balance
    * @param {bigint} fee
    */
    constructor(balance: bigint, fee: bigint);
    __destroy_into_raw(): number;
    ptr: number;
    free(): void;
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
    * @returns {string}
    */
    get message(): string;
    /**
    * @returns {any}
    */
    serialize(): any;
}
/**
*/
export class ChainAssetLockProof {
    static __wrap(ptr: any): any;
    /**
    * @param {any} raw_parameters
    */
    constructor(raw_parameters: any);
    __destroy_into_raw(): number;
    ptr: number;
    free(): void;
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
    static __wrap(ptr: any): any;
    /**
    * @param {any} state_repository
    */
    constructor(state_repository: any);
    __destroy_into_raw(): number;
    ptr: number;
    free(): void;
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
    static __wrap(ptr: any): any;
    __destroy_into_raw(): number;
    ptr: number;
    free(): void;
    /**
    * @returns {number}
    */
    getCurrentProtocolVersion(): number;
}
/**
*/
export class DashPlatformProtocol {
    static __wrap(ptr: any): any;
    /**
    * @param {any} bls_adapter
    * @param {any} state_repository
    * @param {any} entropy_generator
    * @param {number | undefined} maybe_protocol_version
    */
    constructor(bls_adapter: any, state_repository: any, entropy_generator: any, maybe_protocol_version: number | undefined);
    __destroy_into_raw(): number;
    ptr: number;
    free(): void;
    /**
    * @returns {DataContractFacade}
    */
    get dataContract(): DataContractFacade;
    /**
    * @returns {DocumentFacade}
    */
    get document(): DocumentFacade;
    /**
    * @returns {IdentityFacade}
    */
    get identity(): IdentityFacade;
    /**
    * @returns {StateTransitionFacade}
    */
    get stateTransition(): StateTransitionFacade;
    /**
    * @returns {number}
    */
    get protocolVersion(): number;
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
}
/**
*/
export class DataContract {
    static __wrap(ptr: any): any;
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
    * @param {any} raw_parameters
    */
    constructor(raw_parameters: any);
    __destroy_into_raw(): number;
    ptr: number;
    free(): void;
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
    * @returns {DataContract}
    */
    clone(): DataContract;
}
/**
*/
export class DataContractAlreadyPresentError {
    static __wrap(ptr: any): any;
    /**
    * @param {any} data_contract_id
    */
    constructor(data_contract_id: any);
    __destroy_into_raw(): number;
    ptr: number;
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
    * @returns {string}
    */
    get message(): string;
    /**
    * @returns {any}
    */
    serialize(): any;
}
/**
*/
export class DataContractCreateTransition {
    static __wrap(ptr: any): any;
    /**
    * @param {any} raw_parameters
    */
    constructor(raw_parameters: any);
    __destroy_into_raw(): number;
    ptr: number;
    free(): void;
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
    toJSON(skip_signature: boolean | undefined): any;
    /**
    * @param {boolean | undefined} skip_signature
    * @returns {any}
    */
    toBuffer(skip_signature: boolean | undefined): any;
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
    toObject(skip_signature: boolean | undefined): any;
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
    /**
    * @returns {string}
    */
    static get SCHEMA(): string;
    __destroy_into_raw(): number;
    ptr: number;
    free(): void;
}
/**
*/
export class DataContractFacade {
    static __wrap(ptr: any): any;
    __destroy_into_raw(): number;
    ptr: number;
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
    static __wrap(ptr: any): any;
    /**
    * @param {number} protocol_version
    * @param {DataContractValidator} validate_data_contract
    * @param {any | undefined} external_entropy_generator_arg
    */
    constructor(protocol_version: number, validate_data_contract: DataContractValidator, external_entropy_generator_arg: any | undefined);
    __destroy_into_raw(): number;
    ptr: number;
    free(): void;
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
    createFromObject(object: any, skip_validation: boolean | undefined): Promise<DataContract>;
    /**
    * @param {Uint8Array} buffer
    * @param {boolean | undefined} skip_validation
    * @returns {Promise<DataContract>}
    */
    createFromBuffer(buffer: Uint8Array, skip_validation: boolean | undefined): Promise<DataContract>;
    /**
    * @param {DataContract} data_contract
    * @returns {Promise<DataContractCreateTransition>}
    */
    createDataContractCreateTransition(data_contract: DataContract): Promise<DataContractCreateTransition>;
}
/**
*/
export class DataContractGenericError {
    static __wrap(ptr: any): any;
    __destroy_into_raw(): number;
    ptr: number;
    free(): void;
    /**
    * @returns {string}
    */
    getMessage(): string;
}
/**
*/
export class DataContractHaveNewUniqueIndexError {
    static __wrap(ptr: any): any;
    __destroy_into_raw(): number;
    ptr: number;
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
    * @returns {string}
    */
    get message(): string;
    /**
    * @returns {any}
    */
    serialize(): any;
}
/**
*/
export class DataContractImmutablePropertiesUpdateError {
    static __wrap(ptr: any): any;
    __destroy_into_raw(): number;
    ptr: number;
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
    * @returns {string}
    */
    get message(): string;
    /**
    * @returns {any}
    */
    serialize(): any;
}
/**
*/
export class DataContractInvalidIndexDefinitionUpdateError {
    static __wrap(ptr: any): any;
    __destroy_into_raw(): number;
    ptr: number;
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
    * @returns {string}
    */
    get message(): string;
    /**
    * @returns {any}
    */
    serialize(): any;
}
export class DataContractMaxDepthError {
    __destroy_into_raw(): number;
    ptr: number;
    free(): void;
    /**
    * @returns {number}
    */
    getMaxDepth(): number;
    /**
    * @returns {number}
    */
    getSchemaDepth(): number;
    /**
    * @returns {number}
    */
    getCode(): number;
    /**
    * @returns {string}
    */
    get message(): string;
    /**
    * @returns {any}
    */
    serialize(): any;
}
/**
*/
export class DataContractMaxDepthExceedError {
    static __wrap(ptr: any): any;
    __destroy_into_raw(): number;
    ptr: number;
    free(): void;
}
/**
*/
export class DataContractNotPresentError {
    static __wrap(ptr: any): any;
    __destroy_into_raw(): number;
    ptr: number;
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
    * @returns {string}
    */
    get message(): string;
    /**
    * @returns {any}
    */
    serialize(): any;
}
/**
*/
export class DataContractNotPresentNotConsensusError {
    static __wrap(ptr: any): any;
    __destroy_into_raw(): number;
    ptr: number;
    free(): void;
    /**
    * @returns {any}
    */
    getDataContractId(): any;
}
/**
*/
export class DataContractUniqueIndicesChangedError {
    static __wrap(ptr: any): any;
    __destroy_into_raw(): number;
    ptr: number;
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
    * @returns {string}
    */
    get message(): string;
    /**
    * @returns {any}
    */
    serialize(): any;
}
/**
*/
export class DataContractUpdateTransition {
    static __wrap(ptr: any): any;
    /**
    * @param {any} raw_parameters
    */
    constructor(raw_parameters: any);
    __destroy_into_raw(): number;
    ptr: number;
    free(): void;
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
    toJSON(skip_signature: boolean | undefined): any;
    /**
    * @param {boolean | undefined} skip_signature
    * @returns {any}
    */
    toBuffer(skip_signature: boolean | undefined): any;
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
    hash(skip_signature: boolean | undefined): any;
    /**
    * @param {boolean | undefined} skip_signature
    * @returns {any}
    */
    toObject(skip_signature: boolean | undefined): any;
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
    static __wrap(ptr: any): any;
    __destroy_into_raw(): number;
    ptr: number;
    free(): void;
    /**
    * @param {any} raw_data_contract
    * @returns {ValidationResult}
    */
    validate(raw_data_contract: any): ValidationResult;
}
/**
*/
export class DataTrigger {
    static __wrap(ptr: any): any;
    __destroy_into_raw(): number;
    ptr: number;
    free(): void;
    /**
    * @param {any} id
    */
    set dataContractId(arg: any);
    /**
    * @returns {any}
    */
    get dataContractId(): any;
    /**
    * @param {string} id
    */
    set documentType(arg: string);
    /**
    * @returns {string}
    */
    get documentType(): string;
    /**
    * @returns {string}
    */
    get dataTriggerKind(): string;
    /**
    * @param {string} action_string
    */
    set transitionAction(arg: string);
    /**
    * @returns {string}
    */
    get transitionAction(): string;
    /**
    * @param {any} maybe_id
    */
    set topLevelIdentity(arg: any);
    /**
    * @returns {any}
    */
    get topLevelIdentity(): any;
}
/**
*/
export class DataTriggerConditionError {
    static __wrap(ptr: any): any;
    __destroy_into_raw(): number;
    ptr: number;
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
    * @returns {string}
    */
    get message(): string;
    /**
    * @returns {any}
    */
    serialize(): any;
}
/**
*/
export class DataTriggerExecutionContext {
    static __wrap(ptr: any): any;
    /**
    * @param {any} state_repository
    * @param {any} js_owner_id
    * @param {DataContract} data_contract
    * @param {StateTransitionExecutionContext} state_transition_execution_context
    */
    constructor(state_repository: any, js_owner_id: any, data_contract: DataContract, state_transition_execution_context: StateTransitionExecutionContext);
    __destroy_into_raw(): number;
    ptr: number;
    free(): void;
    /**
    * @param {any} owner_id
    */
    set ownerId(arg: any);
    /**
    * @returns {any}
    */
    get ownerId(): any;
    /**
    * @param {DataContract} data_contract
    */
    set dataContract(arg: DataContract);
    /**
    * @returns {DataContract}
    */
    get dataContract(): DataContract;
    /**
    * @returns {StateTransitionExecutionContext}
    */
    get stateTransitionExecutionContext(): StateTransitionExecutionContext;
    /**
    * @param {StateTransitionExecutionContext} context
    */
    set statTransitionExecutionContext(arg: StateTransitionExecutionContext);
}
/**
*/
export class DataTriggerExecutionError {
    static __wrap(ptr: any): any;
    __destroy_into_raw(): number;
    ptr: number;
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
    * @returns {string}
    */
    get message(): string;
    /**
    * @returns {any}
    */
    serialize(): any;
}
/**
*/
export class DataTriggerExecutionResult {
    static __wrap(ptr: any): any;
    __destroy_into_raw(): number;
    ptr: number;
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
    static __wrap(ptr: any): any;
    __destroy_into_raw(): number;
    ptr: number;
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
    * @returns {string}
    */
    get message(): string;
    /**
    * @returns {any}
    */
    serialize(): any;
}
/**
*/
export class Document {
    static __wrap(ptr: any): any;
    /**
    * @param {any} js_raw_document
    * @param {DataContract} js_data_contract
    * @param {any} js_document_type_name
    */
    constructor(js_raw_document: any, js_data_contract: DataContract, js_document_type_name: any);
    __destroy_into_raw(): number;
    ptr: number;
    free(): void;
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
    setRevision(revision: number | undefined): void;
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
    setCreatedAt(created_at: Date | undefined): void;
    /**
    * @param {Date | undefined} updated_at
    */
    setUpdatedAt(updated_at: Date | undefined): void;
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
    static __wrap(ptr: any): any;
    __destroy_into_raw(): number;
    ptr: number;
    free(): void;
    /**
    * @returns {any}
    */
    getDocumentTransition(): any;
}
/**
*/
export class DocumentAlreadyPresentError {
    static __wrap(ptr: any): any;
    __destroy_into_raw(): number;
    ptr: number;
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
    * @returns {string}
    */
    get message(): string;
    /**
    * @returns {any}
    */
    serialize(): any;
}
/**
*/
export class DocumentCreateTransition {
    static __wrap(ptr: any): any;
    /**
    * @param {any} raw_object
    * @param {DataContract} data_contract
    */
    constructor(raw_object: any, data_contract: DataContract);
    __destroy_into_raw(): number;
    ptr: number;
    free(): void;
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
    static __wrap(ptr: any): any;
    __destroy_into_raw(): number;
    ptr: number;
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
    static __wrap(ptr: any): any;
    /**
    * @param {DocumentValidator} document_validator
    * @param {DocumentFactory} document_factory
    * @param {FetchAndValidateDataContractFactory} data_contract_fetcher_and_validator
    */
    constructor(document_validator: DocumentValidator, document_factory: DocumentFactory, data_contract_fetcher_and_validator: FetchAndValidateDataContractFactory);
    __destroy_into_raw(): number;
    ptr: number;
    free(): void;
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
    static __wrap(ptr: any): any;
    /**
    * @param {number} protocol_version
    * @param {DocumentValidator} document_validator
    * @param {any} state_repository
    * @param {any | undefined} external_entropy_generator_arg
    */
    constructor(protocol_version: number, document_validator: DocumentValidator, state_repository: any, external_entropy_generator_arg: any | undefined);
    __destroy_into_raw(): number;
    ptr: number;
    free(): void;
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
    static __wrap(ptr: any): any;
    /**
    * @param {Document} document
    */
    constructor(document: Document);
    __destroy_into_raw(): number;
    ptr: number;
    free(): void;
    /**
    * @returns {Document}
    */
    getDocument(): Document;
}
/**
*/
export class DocumentNotFoundError {
    static __wrap(ptr: any): any;
    __destroy_into_raw(): number;
    ptr: number;
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
    * @returns {string}
    */
    get message(): string;
    /**
    * @returns {any}
    */
    serialize(): any;
}
/**
*/
export class DocumentNotProvidedError {
    static __wrap(ptr: any): any;
    __destroy_into_raw(): number;
    ptr: number;
    free(): void;
    /**
    * @returns {any}
    */
    getDocumentTransition(): any;
}
/**
*/
export class DocumentOwnerIdMismatchError {
    static __wrap(ptr: any): any;
    __destroy_into_raw(): number;
    ptr: number;
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
    * @returns {string}
    */
    get message(): string;
    /**
    * @returns {any}
    */
    serialize(): any;
}
/**
*/
export class DocumentReplaceTransition {
    static __wrap(ptr: any): any;
    /**
    * @param {any} raw_object
    * @param {DataContract} data_contract
    */
    constructor(raw_object: any, data_contract: DataContract);
    __destroy_into_raw(): number;
    ptr: number;
    free(): void;
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
    static __wrap(ptr: any): any;
    __destroy_into_raw(): number;
    ptr: number;
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
    * @returns {string}
    */
    get message(): string;
    /**
    * @returns {any}
    */
    serialize(): any;
}
/**
*/
export class DocumentTimestampsMismatchError {
    static __wrap(ptr: any): any;
    __destroy_into_raw(): number;
    ptr: number;
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
    * @returns {string}
    */
    get message(): string;
    /**
    * @returns {any}
    */
    serialize(): any;
}
/**
*/
export class DocumentTransition {
    static __wrap(ptr: any): any;
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
    __destroy_into_raw(): number;
    ptr: number;
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
    setUpdatedAt(updated_at: Date | undefined): void;
    /**
    * @param {Date | undefined} created_at
    */
    setCreatedAt(created_at: Date | undefined): void;
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
}
/**
*/
export class DocumentTransitions {
    static __wrap(ptr: any): any;
    __destroy_into_raw(): number;
    ptr: number;
    free(): void;
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
    static __wrap(ptr: any): any;
    /**
    * @param {ProtocolVersionValidator} protocol_validator
    */
    constructor(protocol_validator: ProtocolVersionValidator);
    __destroy_into_raw(): number;
    ptr: number;
    free(): void;
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
    static __wrap(ptr: any): any;
    /**
    * @param {any} js_raw_transition
    * @param {Array<any>} data_contracts
    */
    constructor(js_raw_transition: any, data_contracts: Array<any>);
    __destroy_into_raw(): number;
    ptr: number;
    free(): void;
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
    static __wrap(ptr: any): any;
    __destroy_into_raw(): number;
    ptr: number;
    free(): void;
    /**
    * @param {any} number
    */
    set storageFee(arg: bigint);
    /**
    * @returns {bigint}
    */
    get storageFee(): bigint;
    /**
    * @param {any} number
    */
    set processingFee(arg: bigint);
    /**
    * @returns {bigint}
    */
    get processingFee(): bigint;
    /**
    * @param {Array<any>} js_fee_refunds
    */
    set feeRefunds(arg: any[]);
    /**
    * @returns {Array<any>}
    */
    get feeRefunds(): any[];
}
/**
*/
export class DuplicateDocumentTransitionsWithIdsError {
    static __wrap(ptr: any): any;
    __destroy_into_raw(): number;
    ptr: number;
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
    * @returns {string}
    */
    get message(): string;
    /**
    * @returns {any}
    */
    serialize(): any;
}
/**
*/
export class DuplicateDocumentTransitionsWithIndicesError {
    static __wrap(ptr: any): any;
    __destroy_into_raw(): number;
    ptr: number;
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
    * @returns {string}
    */
    get message(): string;
    /**
    * @returns {any}
    */
    serialize(): any;
}
/**
*/
export class DuplicateIndexError {
    static __wrap(ptr: any): any;
    __destroy_into_raw(): number;
    ptr: number;
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
    * @returns {string}
    */
    get message(): string;
    /**
    * @returns {any}
    */
    serialize(): any;
}
/**
*/
export class DuplicateIndexNameError {
    static __wrap(ptr: any): any;
    __destroy_into_raw(): number;
    ptr: number;
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
    * @returns {string}
    */
    get message(): string;
    /**
    * @returns {any}
    */
    serialize(): any;
}
/**
*/
export class DuplicateUniqueIndexError {
    static __wrap(ptr: any): any;
    __destroy_into_raw(): number;
    ptr: number;
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
    * @returns {string}
    */
    get message(): string;
    /**
    * @returns {any}
    */
    serialize(): any;
}
/**
*/
export class DuplicatedIdentityPublicKeyError {
    static __wrap(ptr: any): any;
    __destroy_into_raw(): number;
    ptr: number;
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
    * @returns {string}
    */
    get message(): string;
    /**
    * @returns {any}
    */
    serialize(): any;
}
/**
*/
export class DuplicatedIdentityPublicKeyIdError {
    static __wrap(ptr: any): any;
    __destroy_into_raw(): number;
    ptr: number;
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
    * @returns {string}
    */
    get message(): string;
    /**
    * @returns {any}
    */
    serialize(): any;
}
/**
*/
export class DuplicatedIdentityPublicKeyIdStateError {
    static __wrap(ptr: any): any;
    __destroy_into_raw(): number;
    ptr: number;
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
    * @returns {string}
    */
    get message(): string;
    /**
    * @returns {any}
    */
    serialize(): any;
}
/**
*/
export class DuplicatedIdentityPublicKeyStateError {
    static __wrap(ptr: any): any;
    __destroy_into_raw(): number;
    ptr: number;
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
    * @returns {string}
    */
    get message(): string;
    /**
    * @returns {any}
    */
    serialize(): any;
}
/**
*/
export class ExtendedDocument {
    static __wrap(ptr: any): any;
    /**
    * @param {any} js_raw_document
    * @param {DataContract} js_data_contract
    */
    constructor(js_raw_document: any, js_data_contract: DataContract);
    __destroy_into_raw(): number;
    ptr: number;
    free(): void;
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
    setRevision(rev: number | undefined): void;
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
    setCreatedAt(ts: Date | undefined): void;
    /**
    * @param {Date | undefined} ts
    */
    setUpdatedAt(ts: Date | undefined): void;
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
    static __wrap(ptr: any): any;
    __destroy_into_raw(): number;
    ptr: number;
    free(): void;
    /**
    * @param {any} number
    */
    set storageFee(arg: bigint);
    /**
    * @returns {bigint}
    */
    get storageFee(): bigint;
    /**
    * @param {any} number
    */
    set processingFee(arg: bigint);
    /**
    * @returns {bigint}
    */
    get processingFee(): bigint;
    /**
    * @param {Array<any>} js_fee_refunds
    */
    set feeRefunds(arg: any[]);
    /**
    * @returns {Array<any>}
    */
    get feeRefunds(): any[];
    /**
    * @param {any} number
    */
    set totalRefunds(arg: bigint);
    /**
    * @returns {bigint}
    */
    get totalRefunds(): bigint;
    /**
    * @param {any} number
    */
    set desiredAmount(arg: bigint);
    /**
    * @returns {bigint}
    */
    get desiredAmount(): bigint;
    /**
    * @param {any} number
    */
    set requiredAmount(arg: bigint);
    /**
    * @returns {bigint}
    */
    get requiredAmount(): bigint;
}
/**
*/
export class FetchAndValidateDataContractFactory {
    static __wrap(ptr: any): any;
    /**
    * @param {any} state_repository
    */
    constructor(state_repository: any);
    __destroy_into_raw(): number;
    ptr: number;
    free(): void;
    /**
    * @param {any} js_raw_document
    * @returns {Promise<ValidationResult>}
    */
    validate(js_raw_document: any): Promise<ValidationResult>;
}
/**
*/
export class Identity {
    static __wrap(ptr: any): any;
    /**
    * @param {any} object
    * @returns {Identity}
    */
    static from(object: any): Identity;
    /**
    * @param {any} raw_identity
    */
    constructor(raw_identity: any);
    __destroy_into_raw(): number;
    ptr: number;
    free(): void;
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
    addPublicKeys(...js_public_keys: any[]): void;
    /**
    * @returns {number}
    */
    getPublicKeyMaxId(): number;
}
/**
*/
export class IdentityAlreadyExistsError {
    static __wrap(ptr: any): any;
    __destroy_into_raw(): number;
    ptr: number;
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
    * @returns {string}
    */
    get message(): string;
    /**
    * @returns {any}
    */
    serialize(): any;
}
/**
*/
export class IdentityAssetLockProofLockedTransactionMismatchError {
    static __wrap(ptr: any): any;
    __destroy_into_raw(): number;
    ptr: number;
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
    * @returns {string}
    */
    get message(): string;
    /**
    * @returns {any}
    */
    serialize(): any;
}
/**
*/
export class IdentityAssetLockTransactionIsNotFoundError {
    static __wrap(ptr: any): any;
    __destroy_into_raw(): number;
    ptr: number;
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
    * @returns {string}
    */
    get message(): string;
    /**
    * @returns {any}
    */
    serialize(): any;
}
/**
*/
export class IdentityAssetLockTransactionOutPointAlreadyExistsError {
    static __wrap(ptr: any): any;
    __destroy_into_raw(): number;
    ptr: number;
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
    * @returns {string}
    */
    get message(): string;
    /**
    * @returns {any}
    */
    serialize(): any;
}
/**
*/
export class IdentityAssetLockTransactionOutputNotFoundError {
    static __wrap(ptr: any): any;
    __destroy_into_raw(): number;
    ptr: number;
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
    * @returns {string}
    */
    get message(): string;
    /**
    * @returns {any}
    */
    serialize(): any;
}
/**
*/
export class IdentityCreateTransition {
    static __wrap(ptr: any): any;
    /**
    * @param {any} raw_parameters
    */
    constructor(raw_parameters: any);
    __destroy_into_raw(): number;
    ptr: number;
    free(): void;
    /**
    * @param {any} asset_lock_proof
    */
    setAssetLockProof(asset_lock_proof: any): void;
    /**
    * @returns {any}
    */
    get assetLockProof(): any;
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
    * @returns {any[]}
    */
    get publicKeys(): any[];
    /**
    * @returns {number}
    */
    getType(): number;
    /**
    * @returns {any}
    */
    get identityId(): any;
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
    setSignature(signature: Uint8Array | undefined): void;
}
/**
*/
export class IdentityCreateTransitionBasicValidator {
    static __wrap(ptr: any): any;
    /**
    * @param {any} state_repository
    * @param {any} js_bls
    */
    constructor(state_repository: any, js_bls: any);
    __destroy_into_raw(): number;
    ptr: number;
    free(): void;
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
    static __wrap(ptr: any): any;
    /**
    * @param {any} state_repository
    */
    constructor(state_repository: any);
    __destroy_into_raw(): number;
    ptr: number;
    free(): void;
    /**
    * @param {IdentityCreateTransition} state_transition
    * @returns {Promise<ValidationResult>}
    */
    validate(state_transition: IdentityCreateTransition): Promise<ValidationResult>;
}
/**
*/
export class IdentityFacade {
    static __wrap(ptr: any): any;
    __destroy_into_raw(): number;
    ptr: number;
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
    static __wrap(ptr: any): any;
    /**
    * @param {number} protocol_version
    * @param {IdentityValidator} identity_validator
    */
    constructor(protocol_version: number, identity_validator: IdentityValidator);
    __destroy_into_raw(): number;
    ptr: number;
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
    static __wrap(ptr: any): any;
    __destroy_into_raw(): number;
    ptr: number;
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
    * @returns {string}
    */
    get message(): string;
    /**
    * @returns {any}
    */
    serialize(): any;
}
/**
*/
export class IdentityNotFoundError {
    static __wrap(ptr: any): any;
    /**
    * @param {any} identity_id
    */
    constructor(identity_id: any);
    __destroy_into_raw(): number;
    ptr: number;
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
    * @returns {string}
    */
    get message(): string;
    /**
    * @returns {any}
    */
    serialize(): any;
}
/**
*/
export class IdentityPublicKey {
    static __wrap(ptr: any): any;
    /**
    * @param {any} raw_public_key
    */
    constructor(raw_public_key: any);
    __destroy_into_raw(): number;
    ptr: number;
    free(): void;
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
    static __wrap(ptr: any): any;
    __destroy_into_raw(): number;
    ptr: number;
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
    * @returns {string}
    */
    get message(): string;
    /**
    * @returns {any}
    */
    serialize(): any;
}
/**
*/
export class IdentityPublicKeyIsDisabledError {
    static __wrap(ptr: any): any;
    __destroy_into_raw(): number;
    ptr: number;
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
    * @returns {string}
    */
    get message(): string;
    /**
    * @returns {any}
    */
    serialize(): any;
}
/**
*/
export class IdentityPublicKeyIsReadOnlyError {
    static __wrap(ptr: any): any;
    __destroy_into_raw(): number;
    ptr: number;
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
    * @returns {string}
    */
    get message(): string;
    /**
    * @returns {any}
    */
    serialize(): any;
}
/**
*/
export class IdentityPublicKeyWithWitness {
    static __wrap(ptr: any): any;
    /**
    * @param {any} raw_public_key
    */
    constructor(raw_public_key: any);
    __destroy_into_raw(): number;
    ptr: number;
    free(): void;
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
    static __wrap(ptr: any): any;
    /**
    * @param {any} raw_parameters
    */
    constructor(raw_parameters: any);
    __destroy_into_raw(): number;
    ptr: number;
    free(): void;
    /**
    * @param {any} asset_lock_proof
    */
    setAssetLockProof(asset_lock_proof: any): void;
    /**
    * @returns {any}
    */
    get assetLockProof(): any;
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
    get identityId(): any;
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
    setSignature(signature: Uint8Array | undefined): void;
}
/**
*/
export class IdentityTopUpTransitionBasicValidator {
    static __wrap(ptr: any): any;
    /**
    * @param {any} state_repository
    */
    constructor(state_repository: any);
    __destroy_into_raw(): number;
    ptr: number;
    free(): void;
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
    static __wrap(ptr: any): any;
    /**
    * @param {any} state_repository
    */
    constructor(state_repository: any);
    __destroy_into_raw(): number;
    ptr: number;
    free(): void;
    /**
    * @param {IdentityTopUpTransition} state_transition
    * @returns {Promise<ValidationResult>}
    */
    validate(state_transition: IdentityTopUpTransition): Promise<ValidationResult>;
}
/**
*/
export class IdentityUpdatePublicKeysValidator {
    static __wrap(ptr: any): any;
    __destroy_into_raw(): number;
    ptr: number;
    free(): void;
    /**
    * @param {any[]} raw_public_keys
    * @returns {ValidationResult}
    */
    validate(raw_public_keys: any[]): ValidationResult;
}
/**
*/
export class IdentityUpdateTransition {
    static __wrap(ptr: any): any;
    /**
    * @param {any} raw_parameters
    */
    constructor(raw_parameters: any);
    __destroy_into_raw(): number;
    ptr: number;
    free(): void;
    /**
    * @param {any[] | undefined} public_keys
    */
    setPublicKeysToAdd(public_keys: any[] | undefined): void;
    /**
    * @returns {any[]}
    */
    getPublicKeysToAdd(): any[];
    /**
    * @returns {any[]}
    */
    get addPublicKeys(): any[];
    /**
    * @returns {any[]}
    */
    getPublicKeyIdsToDisable(): any[];
    /**
    * @param {Uint32Array | undefined} public_key_ids
    */
    setPublicKeyIdsToDisable(public_key_ids: Uint32Array | undefined): void;
    /**
    * @returns {Date | undefined}
    */
    getPublicKeysDisabledAt(): Date | undefined;
    /**
    * @param {Date | undefined} timestamp
    */
    setPublicKeysDisabledAt(timestamp: Date | undefined): void;
    /**
    * @returns {number}
    */
    getType(): number;
    /**
    * @returns {any}
    */
    get identityId(): any;
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
    setSignaturePublicKeyId(key_id: number | undefined): void;
    /**
    * @returns {any}
    */
    getSignature(): any;
    /**
    * @param {Uint8Array | undefined} signature
    */
    setSignature(signature: Uint8Array | undefined): void;
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
}
/**
*/
export class IdentityUpdateTransitionBasicValidator {
    static __wrap(ptr: any): any;
    /**
    * @param {any} js_bls
    */
    constructor(js_bls: any);
    __destroy_into_raw(): number;
    ptr: number;
    free(): void;
    /**
    * @param {any} raw_state_transition
    * @returns {ValidationResult}
    */
    validate(raw_state_transition: any): ValidationResult;
}
/**
*/
export class IdentityUpdateTransitionStateValidator {
    static __wrap(ptr: any): any;
    /**
    * @param {any} state_repository
    */
    constructor(state_repository: any);
    __destroy_into_raw(): number;
    ptr: number;
    free(): void;
    /**
    * @param {IdentityUpdateTransition} state_transition
    * @returns {Promise<ValidationResult>}
    */
    validate(state_transition: IdentityUpdateTransition): Promise<ValidationResult>;
}
/**
*/
export class IdentityValidator {
    static __wrap(ptr: any): any;
    /**
    * @param {any} bls
    */
    constructor(bls: any);
    __destroy_into_raw(): number;
    ptr: number;
    free(): void;
    /**
    * @param {any} raw_identity
    * @returns {ValidationResult}
    */
    validate(raw_identity: any): ValidationResult;
}
/**
*/
export class IncompatibleDataContractSchemaError {
    static __wrap(ptr: any): any;
    __destroy_into_raw(): number;
    ptr: number;
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
    * @returns {string}
    */
    get message(): string;
    /**
    * @returns {any}
    */
    serialize(): any;
}
/**
*/
export class IncompatibleProtocolVersionError {
    static __wrap(ptr: any): any;
    __destroy_into_raw(): number;
    ptr: number;
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
    * @returns {string}
    */
    get message(): string;
    /**
    * @returns {any}
    */
    serialize(): any;
}
/**
*/
export class IncompatibleRe2PatternError {
    static __wrap(ptr: any): any;
    __destroy_into_raw(): number;
    ptr: number;
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
    * @returns {string}
    */
    get message(): string;
    /**
    * @returns {any}
    */
    serialize(): any;
}
/**
*/
export class InconsistentCompoundIndexDataError {
    static __wrap(ptr: any): any;
    __destroy_into_raw(): number;
    ptr: number;
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
    * @returns {string}
    */
    get message(): string;
    /**
    * @returns {any}
    */
    serialize(): any;
}
/**
*/
export class IndexDefinition {
    __destroy_into_raw(): number;
    ptr: number;
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
    static __wrap(ptr: any): any;
    __destroy_into_raw(): number;
    ptr: number;
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
    static __wrap(ptr: any): any;
    /**
    * @param {any} raw_parameters
    */
    constructor(raw_parameters: any);
    __destroy_into_raw(): number;
    ptr: number;
    free(): void;
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
    static __wrap(ptr: any): any;
    /**
    * @param {any} state_repository
    */
    constructor(state_repository: any);
    __destroy_into_raw(): number;
    ptr: number;
    free(): void;
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
    static __wrap(ptr: any): any;
    __destroy_into_raw(): number;
    ptr: number;
    free(): void;
}
/**
*/
export class InvalidActionNameError {
    static __wrap(ptr: any): any;
    /**
    * @param {any[]} actions
    */
    constructor(actions: any[]);
    __destroy_into_raw(): number;
    ptr: number;
    free(): void;
    /**
    * @returns {any[]}
    */
    getActions(): any[];
}
export class InvalidActiontError {
    /**
    * @param {any} action
    */
    constructor(action: any);
    __destroy_into_raw(): number;
    ptr: number;
    free(): void;
}
/**
*/
export class InvalidAssetLockProofCoreChainHeightError {
    static __wrap(ptr: any): any;
    __destroy_into_raw(): number;
    ptr: number;
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
    * @returns {string}
    */
    get message(): string;
    /**
    * @returns {any}
    */
    serialize(): any;
}
/**
*/
export class InvalidAssetLockProofTransactionHeightError {
    static __wrap(ptr: any): any;
    __destroy_into_raw(): number;
    ptr: number;
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
    * @returns {string}
    */
    get message(): string;
    /**
    * @returns {any}
    */
    serialize(): any;
}
/**
*/
export class InvalidAssetLockTransactionOutputReturnSizeError {
    static __wrap(ptr: any): any;
    __destroy_into_raw(): number;
    ptr: number;
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
    * @returns {string}
    */
    get message(): string;
    /**
    * @returns {any}
    */
    serialize(): any;
}
/**
*/
export class InvalidCompoundIndexError {
    static __wrap(ptr: any): any;
    __destroy_into_raw(): number;
    ptr: number;
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
    * @returns {string}
    */
    get message(): string;
    /**
    * @returns {any}
    */
    serialize(): any;
}
/**
*/
export class InvalidDataContractError {
    static __wrap(ptr: any): any;
    __destroy_into_raw(): number;
    ptr: number;
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
    static __wrap(ptr: any): any;
    __destroy_into_raw(): number;
    ptr: number;
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
    * @returns {string}
    */
    get message(): string;
    /**
    * @returns {any}
    */
    serialize(): any;
}
/**
*/
export class InvalidDataContractVersionError {
    static __wrap(ptr: any): any;
    __destroy_into_raw(): number;
    ptr: number;
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
    * @returns {string}
    */
    get message(): string;
    /**
    * @returns {any}
    */
    serialize(): any;
}
/**
*/
export class InvalidDocumentActionError {
    static __wrap(ptr: any): any;
    __destroy_into_raw(): number;
    ptr: number;
    free(): void;
    /**
    * @returns {any}
    */
    getDocumentTransition(): any;
}
/**
*/
export class InvalidDocumentError {
    static __wrap(ptr: any): any;
    /**
    * @param {any} raw_document
    * @param {any[]} errors
    */
    constructor(raw_document: any, errors: any[]);
    __destroy_into_raw(): number;
    ptr: number;
    free(): void;
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
    static __wrap(ptr: any): any;
    __destroy_into_raw(): number;
    ptr: number;
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
    * @returns {string}
    */
    get message(): string;
    /**
    * @returns {any}
    */
    serialize(): any;
}
/**
*/
export class InvalidDocumentTransitionActionError {
    static __wrap(ptr: any): any;
    __destroy_into_raw(): number;
    ptr: number;
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
    * @returns {string}
    */
    get message(): string;
    /**
    * @returns {any}
    */
    serialize(): any;
}
/**
*/
export class InvalidDocumentTransitionIdError {
    static __wrap(ptr: any): any;
    __destroy_into_raw(): number;
    ptr: number;
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
    * @returns {string}
    */
    get message(): string;
    /**
    * @returns {any}
    */
    serialize(): any;
}
/**
*/
export class InvalidDocumentTypeError {
    static __wrap(ptr: any): any;
    __destroy_into_raw(): number;
    ptr: number;
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
    * @returns {string}
    */
    get message(): string;
    /**
    * @returns {any}
    */
    serialize(): any;
}
/**
*/
export class InvalidDocumentTypeInDataContractError {
    static __wrap(ptr: any): any;
    /**
    * @param {string} doc_type
    * @param {any} data_contract_id
    */
    constructor(doc_type: string, data_contract_id: any);
    __destroy_into_raw(): number;
    ptr: number;
    free(): void;
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
    static __wrap(ptr: any): any;
    __destroy_into_raw(): number;
    ptr: number;
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
    * @returns {string}
    */
    get message(): string;
    /**
    * @returns {any}
    */
    serialize(): any;
}
/**
*/
export class InvalidIdentityAssetLockTransactionError {
    static __wrap(ptr: any): any;
    __destroy_into_raw(): number;
    ptr: number;
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
    * @returns {string}
    */
    get message(): string;
    /**
    * @returns {any}
    */
    serialize(): any;
}
/**
*/
export class InvalidIdentityAssetLockTransactionOutputError {
    static __wrap(ptr: any): any;
    __destroy_into_raw(): number;
    ptr: number;
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
    * @returns {string}
    */
    get message(): string;
    /**
    * @returns {any}
    */
    serialize(): any;
}
/**
*/
export class InvalidIdentityCreditWithdrawalTransitionCoreFeeError {
    static __wrap(ptr: any): any;
    __destroy_into_raw(): number;
    ptr: number;
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
    * @returns {string}
    */
    get message(): string;
    /**
    * @returns {any}
    */
    serialize(): any;
}
/**
*/
export class InvalidIdentityCreditWithdrawalTransitionOutputScriptError {
    static __wrap(ptr: any): any;
    __destroy_into_raw(): number;
    ptr: number;
    free(): void;
    /**
    * @returns {number}
    */
    getCode(): number;
    /**
    * @returns {string}
    */
    get message(): string;
    /**
    * @returns {any}
    */
    serialize(): any;
}
/**
*/
export class InvalidIdentityError {
    static __wrap(ptr: any): any;
    __destroy_into_raw(): number;
    ptr: number;
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
    static __wrap(ptr: any): any;
    __destroy_into_raw(): number;
    ptr: number;
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
    * @returns {string}
    */
    get message(): string;
    /**
    * @returns {any}
    */
    serialize(): any;
}
/**
*/
export class InvalidIdentityPublicKeyDataError {
    static __wrap(ptr: any): any;
    __destroy_into_raw(): number;
    ptr: number;
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
    * @returns {string}
    */
    get message(): string;
    /**
    * @returns {any}
    */
    serialize(): any;
}
/**
*/
export class InvalidIdentityPublicKeyIdError {
    static __wrap(ptr: any): any;
    __destroy_into_raw(): number;
    ptr: number;
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
    * @returns {string}
    */
    get message(): string;
    /**
    * @returns {any}
    */
    serialize(): any;
}
/**
*/
export class InvalidIdentityPublicKeySecurityLevelError {
    static __wrap(ptr: any): any;
    __destroy_into_raw(): number;
    ptr: number;
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
    * @returns {string}
    */
    get message(): string;
    /**
    * @returns {any}
    */
    serialize(): any;
}
/**
*/
export class InvalidIdentityPublicKeyTypeError {
    static __wrap(ptr: any): any;
    /**
    * @param {number} key_type
    */
    constructor(key_type: number);
    __destroy_into_raw(): number;
    ptr: number;
    free(): void;
    /**
    * @returns {number}
    */
    getPublicKeyType(): number;
    /**
    * @returns {number}
    */
    getCode(): number;
    /**
    * @returns {string}
    */
    get message(): string;
    /**
    * @returns {any}
    */
    serialize(): any;
}
/**
*/
export class InvalidIdentityRevisionError {
    static __wrap(ptr: any): any;
    __destroy_into_raw(): number;
    ptr: number;
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
    * @returns {string}
    */
    get message(): string;
    /**
    * @returns {any}
    */
    serialize(): any;
}
/**
*/
export class InvalidIndexPropertyTypeError {
    static __wrap(ptr: any): any;
    __destroy_into_raw(): number;
    ptr: number;
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
    * @returns {string}
    */
    get message(): string;
    /**
    * @returns {any}
    */
    serialize(): any;
}
/**
*/
export class InvalidIndexedPropertyConstraintError {
    static __wrap(ptr: any): any;
    __destroy_into_raw(): number;
    ptr: number;
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
    * @returns {string}
    */
    get message(): string;
    /**
    * @returns {any}
    */
    serialize(): any;
}
/**
*/
export class InvalidInitialRevisionError {
    static __wrap(ptr: any): any;
    /**
    * @param {ExtendedDocument} document
    */
    constructor(document: ExtendedDocument);
    __destroy_into_raw(): number;
    ptr: number;
    free(): void;
    /**
    * @returns {ExtendedDocument}
    */
    getDocument(): ExtendedDocument;
}
/**
*/
export class InvalidInstantAssetLockProofError {
    static __wrap(ptr: any): any;
    __destroy_into_raw(): number;
    ptr: number;
    free(): void;
    /**
    * @returns {number}
    */
    getCode(): number;
    /**
    * @returns {string}
    */
    get message(): string;
    /**
    * @returns {any}
    */
    serialize(): any;
}
/**
*/
export class InvalidInstantAssetLockProofSignatureError {
    static __wrap(ptr: any): any;
    __destroy_into_raw(): number;
    ptr: number;
    free(): void;
    /**
    * @returns {number}
    */
    getCode(): number;
    /**
    * @returns {string}
    */
    get message(): string;
    /**
    * @returns {any}
    */
    serialize(): any;
}
/**
*/
export class InvalidJsonSchemaRefError {
    static __wrap(ptr: any): any;
    __destroy_into_raw(): number;
    ptr: number;
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
    * @returns {string}
    */
    get message(): string;
    /**
    * @returns {any}
    */
    serialize(): any;
}
/**
*/
export class InvalidSignaturePublicKeySecurityLevelError {
    static __wrap(ptr: any): any;
    __destroy_into_raw(): number;
    ptr: number;
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
    * @returns {string}
    */
    get message(): string;
    /**
    * @returns {any}
    */
    serialize(): any;
}
/**
*/
export class InvalidStateTransitionError {
    static __wrap(ptr: any): any;
    /**
    * @param {any[]} error_buffers
    * @param {any} raw_state_transition
    */
    constructor(error_buffers: any[], raw_state_transition: any);
    __destroy_into_raw(): number;
    ptr: number;
    free(): void;
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
    static __wrap(ptr: any): any;
    __destroy_into_raw(): number;
    ptr: number;
    free(): void;
    /**
    * @returns {number}
    */
    getCode(): number;
    /**
    * @returns {string}
    */
    get message(): string;
    /**
    * @returns {any}
    */
    serialize(): any;
}
/**
*/
export class InvalidStateTransitionTypeError {
    static __wrap(ptr: any): any;
    /**
    * @param {number} transition_type
    */
    constructor(transition_type: number);
    __destroy_into_raw(): number;
    ptr: number;
    free(): void;
    /**
    * @returns {number}
    */
    getType(): number;
    /**
    * @returns {number}
    */
    getCode(): number;
    /**
    * @returns {string}
    */
    get message(): string;
    /**
    * @returns {any}
    */
    serialize(): any;
}
/**
*/
export class JsonSchemaCompilationError {
    static __wrap(ptr: any): any;
    __destroy_into_raw(): number;
    ptr: number;
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
    * @returns {string}
    */
    get message(): string;
    /**
    * @returns {any}
    */
    serialize(): any;
}
/**
*/
export class JsonSchemaError {
    static __wrap(ptr: any): any;
    toJSON(): {
        message: string;
    };
    toString(): string;
    /**
    * @returns {string}
    */
    toString(): string;
    __destroy_into_raw(): number;
    ptr: number;
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
    get message(): string;
    /**
    * @returns {any}
    */
    serialize(): any;
}
/**
*/
export class JsonSchemaValidator {
    static __wrap(ptr: any): any;
    /**
    * @param {any} schema_js
    * @param {any} definitions
    */
    constructor(schema_js: any, definitions: any);
    __destroy_into_raw(): number;
    ptr: number;
    free(): void;
}
/**
*/
export class MaxIdentityPublicKeyLimitReachedError {
    static __wrap(ptr: any): any;
    __destroy_into_raw(): number;
    ptr: number;
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
    * @returns {string}
    */
    get message(): string;
    /**
    * @returns {any}
    */
    serialize(): any;
}
/**
*/
export class Metadata {
    static __wrap(ptr: any): any;
    /**
    * @param {any} object
    * @returns {Metadata}
    */
    static from(object: any): Metadata;
    /**
    * @param {any} options
    */
    constructor(options: any);
    __destroy_into_raw(): number;
    ptr: number;
    free(): void;
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
    static __wrap(ptr: any): any;
    /**
    * @param {any[]} documents
    */
    constructor(documents: any[]);
    __destroy_into_raw(): number;
    ptr: number;
    free(): void;
    /**
    * @returns {any[]}
    */
    getDocuments(): any[];
}
/**
*/
export class MissingDataContractIdError {
    static __wrap(ptr: any): any;
    __destroy_into_raw(): number;
    ptr: number;
    free(): void;
    /**
    * @returns {number}
    */
    getCode(): number;
    /**
    * @returns {string}
    */
    get message(): string;
    /**
    * @returns {any}
    */
    serialize(): any;
}
/**
*/
export class MissingDocumentTransitionActionError {
    static __wrap(ptr: any): any;
    __destroy_into_raw(): number;
    ptr: number;
    free(): void;
    /**
    * @returns {number}
    */
    getCode(): number;
    /**
    * @returns {string}
    */
    get message(): string;
    /**
    * @returns {any}
    */
    serialize(): any;
}
/**
*/
export class MissingDocumentTransitionTypeError {
    static __wrap(ptr: any): any;
    __destroy_into_raw(): number;
    ptr: number;
    free(): void;
    /**
    * @returns {number}
    */
    getCode(): number;
    /**
    * @returns {string}
    */
    get message(): string;
    /**
    * @returns {any}
    */
    serialize(): any;
}
/**
*/
export class MissingDocumentTypeError {
    static __wrap(ptr: any): any;
    __destroy_into_raw(): number;
    ptr: number;
    free(): void;
    /**
    * @returns {number}
    */
    getCode(): number;
    /**
    * @returns {string}
    */
    get message(): string;
    /**
    * @returns {any}
    */
    serialize(): any;
}
/**
*/
export class MissingMasterPublicKeyError {
    static __wrap(ptr: any): any;
    __destroy_into_raw(): number;
    ptr: number;
    free(): void;
    /**
    * @returns {number}
    */
    getCode(): number;
    /**
    * @returns {string}
    */
    get message(): string;
    /**
    * @returns {any}
    */
    serialize(): any;
}
/**
*/
export class MissingPublicKeyError {
    static __wrap(ptr: any): any;
    __destroy_into_raw(): number;
    ptr: number;
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
    * @returns {string}
    */
    get message(): string;
    /**
    * @returns {any}
    */
    serialize(): any;
}
/**
*/
export class MissingStateTransitionTypeError {
    static __wrap(ptr: any): any;
    __destroy_into_raw(): number;
    ptr: number;
    free(): void;
    /**
    * @returns {number}
    */
    getCode(): number;
    /**
    * @returns {string}
    */
    get message(): string;
    /**
    * @returns {any}
    */
    serialize(): any;
}
/**
*/
export class NoDocumentsSuppliedError {
    static __wrap(ptr: any): any;
    __destroy_into_raw(): number;
    ptr: number;
    free(): void;
}
/**
*/
export class NonConsensusErrorWasm {
    __destroy_into_raw(): number;
    ptr: number;
    free(): void;
}
/**
*/
export class NotImplementedIdentityCreditWithdrawalTransitionPoolingError {
    static __wrap(ptr: any): any;
    __destroy_into_raw(): number;
    ptr: number;
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
    * @returns {string}
    */
    get message(): string;
    /**
    * @returns {any}
    */
    serialize(): any;
}
/**
*/
export class Operation {
    __destroy_into_raw(): number;
    ptr: number;
    free(): void;
}
/**
*/
export class PlatformValueError {
    static __wrap(ptr: any): any;
    toJSON(): {};
    toString(): string;
    /**
    * @returns {string}
    */
    toString(): string;
    __destroy_into_raw(): number;
    ptr: number;
    free(): void;
    /**
    * @returns {string}
    */
    getMessage(): string;
}
/**
*/
export class PreCalculatedOperation {
    static __wrap(ptr: any): any;
    /**
    * @param {DummyFeesResult} dummy_fee_result
    * @returns {PreCalculatedOperation}
    */
    static fromFee(dummy_fee_result: DummyFeesResult): PreCalculatedOperation;
    /**
    * @param {any} storage_cost
    * @param {any} processing_cost
    * @param {Array<any>} js_fee_refunds
    */
    constructor(storage_cost: any, processing_cost: any, js_fee_refunds: Array<any>);
    __destroy_into_raw(): number;
    ptr: number;
    free(): void;
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
    get refunds(): any[];
    /**
    * @returns {Array<any> | undefined}
    */
    refunds_as_objects(): Array<any> | undefined;
    /**
    * @returns {any}
    */
    toJSON(): any;
}
/**
*/
export class ProtocolVersionParsingError {
    static __wrap(ptr: any): any;
    /**
    * @param {string} parsing_error
    */
    constructor(parsing_error: string);
    __destroy_into_raw(): number;
    ptr: number;
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
    * @returns {string}
    */
    get message(): string;
}
/**
*/
export class ProtocolVersionValidator {
    static __wrap(ptr: any): any;
    /**
    * @param {any} options
    */
    constructor(options: any);
    __destroy_into_raw(): number;
    ptr: number;
    free(): void;
    /**
    * @param {number} version
    * @returns {ValidationResult}
    */
    validate(version: number): ValidationResult;
}
/**
*/
export class PublicKeyIsDisabledError {
    static __wrap(ptr: any): any;
    __destroy_into_raw(): number;
    ptr: number;
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
    * @returns {string}
    */
    get message(): string;
    /**
    * @returns {any}
    */
    serialize(): any;
}
/**
*/
export class PublicKeySecurityLevelNotMetError {
    static __wrap(ptr: any): any;
    __destroy_into_raw(): number;
    ptr: number;
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
    * @returns {string}
    */
    get message(): string;
    /**
    * @returns {any}
    */
    serialize(): any;
}
/**
*/
export class PublicKeyValidationError {
    __destroy_into_raw(): number;
    ptr: number;
    free(): void;
    /**
    * @returns {any}
    */
    get message(): any;
}
/**
*/
export class PublicKeysSignaturesValidator {
    static __wrap(ptr: any): any;
    /**
    * @param {any} bls
    */
    constructor(bls: any);
    __destroy_into_raw(): number;
    ptr: number;
    free(): void;
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
    static __wrap(ptr: any): any;
    /**
    * @param {any} adapter
    */
    constructor(adapter: any);
    __destroy_into_raw(): number;
    ptr: number;
    free(): void;
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
    static __wrap(ptr: any): any;
    /**
    * @param {any} value_size
    */
    constructor(value_size: any);
    __destroy_into_raw(): number;
    ptr: number;
    free(): void;
    /**
    * @returns {bigint}
    */
    get processingCost(): bigint;
    /**
    * @returns {bigint}
    */
    get storageCost(): bigint;
    /**
    * @returns {Array<any> | undefined}
    */
    get refunds(): any[];
    /**
    * @returns {any}
    */
    toJSON(): any;
}
/**
*/
export class Refunds {
    static __wrap(ptr: any): any;
    __destroy_into_raw(): number;
    ptr: number;
    free(): void;
    /**
    * @returns {any}
    */
    get identifier(): any;
    /**
    * @returns {Map<any, any>}
    */
    get credits_per_epoch(): Map<any, any>;
    /**
    * @returns {any}
    */
    toObject(): any;
}
/**
*/
export class RevisionAbsentError {
    static __wrap(ptr: any): any;
    /**
    * @param {ExtendedDocument} extended_document
    */
    constructor(extended_document: ExtendedDocument);
    __destroy_into_raw(): number;
    ptr: number;
    free(): void;
    /**
    * @returns {ExtendedDocument}
    */
    getDocument(): ExtendedDocument;
}
/**
*/
export class SerializedObjectParsingError {
    static __wrap(ptr: any): any;
    __destroy_into_raw(): number;
    ptr: number;
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
    * @returns {string}
    */
    get message(): string;
    /**
    * @returns {any}
    */
    serialize(): any;
}
/**
*/
export class SignatureVerificationOperation {
    static __wrap(ptr: any): any;
    /**
    * @param {number} signature_type
    */
    constructor(signature_type: number);
    __destroy_into_raw(): number;
    ptr: number;
    free(): void;
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
    get refunds(): any[];
    /**
    * @returns {any}
    */
    toJSON(): any;
}
/**
*/
export class StateTransitionExecutionContext {
    static __wrap(ptr: any): any;
    __destroy_into_raw(): number;
    ptr: number;
    free(): void;
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
    static __wrap(ptr: any): any;
    __destroy_into_raw(): number;
    ptr: number;
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
    static __wrap(ptr: any): any;
    /**
    * @param {any} state_repository
    * @param {any} bls_adapter
    */
    constructor(state_repository: any, bls_adapter: any);
    __destroy_into_raw(): number;
    ptr: number;
    free(): void;
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
    static __wrap(ptr: any): any;
    /**
    * @param {any} state_repository
    */
    constructor(state_repository: any);
    __destroy_into_raw(): number;
    ptr: number;
    free(): void;
    /**
    * @param {any} state_transition
    * @returns {Promise<ValidationResult>}
    */
    validate(state_transition: any): Promise<ValidationResult>;
}
/**
*/
export class StateTransitionMaxSizeExceededError {
    static __wrap(ptr: any): any;
    __destroy_into_raw(): number;
    ptr: number;
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
    * @returns {string}
    */
    get message(): string;
    /**
    * @returns {any}
    */
    serialize(): any;
}
/**
*/
export class SystemPropertyIndexAlreadyPresentError {
    static __wrap(ptr: any): any;
    __destroy_into_raw(): number;
    ptr: number;
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
    * @returns {string}
    */
    get message(): string;
    /**
    * @returns {any}
    */
    serialize(): any;
}
/**
*/
export class TryingToReplaceImmutableDocumentError {
    static __wrap(ptr: any): any;
    /**
    * @param {ExtendedDocument} extended_document
    */
    constructor(extended_document: ExtendedDocument);
    __destroy_into_raw(): number;
    ptr: number;
    free(): void;
}
/**
*/
export class UndefinedIndexPropertyError {
    static __wrap(ptr: any): any;
    __destroy_into_raw(): number;
    ptr: number;
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
    * @returns {string}
    */
    get message(): string;
    /**
    * @returns {any}
    */
    serialize(): any;
}
/**
*/
export class UniqueIndicesLimitReachedError {
    static __wrap(ptr: any): any;
    __destroy_into_raw(): number;
    ptr: number;
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
    * @returns {string}
    */
    get message(): string;
    /**
    * @returns {any}
    */
    serialize(): any;
}
/**
*/
export class UnknownAssetLockProofTypeError {
    static __wrap(ptr: any): any;
    __destroy_into_raw(): number;
    ptr: number;
    free(): void;
    /**
    * @returns {number | undefined}
    */
    getType(): number | undefined;
}
/**
*/
export class UnsupportedProtocolVersionError {
    static __wrap(ptr: any): any;
    __destroy_into_raw(): number;
    ptr: number;
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
    * @returns {string}
    */
    get message(): string;
    /**
    * @returns {any}
    */
    serialize(): any;
}
/**
*/
export class ValidationResult {
    static __wrap(ptr: any): any;
    /**
    * @param {any[] | undefined} errors_option
    */
    constructor(errors_option: any[] | undefined);
    __destroy_into_raw(): number;
    ptr: number;
    free(): void;
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
    static __wrap(ptr: any): any;
    toJSON(): {
        message: string;
    };
    toString(): string;
    __destroy_into_raw(): number;
    ptr: number;
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
    * @returns {string}
    */
    get message(): string;
    /**
    * @returns {any}
    */
    serialize(): any;
}
/**
*/
export class WrongPublicKeyPurposeError {
    static __wrap(ptr: any): any;
    __destroy_into_raw(): number;
    ptr: number;
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
    * @returns {string}
    */
    get message(): string;
    /**
    * @returns {any}
    */
    serialize(): any;
}
export default init;
export function initSync(module: any): any;
declare function init(input: any): Promise<any>;
