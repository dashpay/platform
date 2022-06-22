/**
 * @interface StateRepository
 * @classdesc StateRepository interface definition
 */

/**
 * Fetch Data Contract by ID
 *
 * @async
 * @method
 * @name StateRepository#fetchDataContract
 * @param {Identifier} id
 * @param {StateTransitionExecutionContext} [StateTransitionExecutionContext]
 * @returns {Promise<DataContract|null>}
 */

/**
 * Store Data Contract
 *
 * @async
 * @method
 * @name StateRepository#storeDataContract
 * @param {DataContract} dataContract
 * @param {StateTransitionExecutionContext} [StateTransitionExecutionContext]
 * @returns {Promise<void>}
 */

/**
 * Fetch Documents by Data Contract ID and type
 *
 * @async
 * @method
 * @name StateRepository#fetchDocuments
 * @param {Identifier} contractId
 * @param {string} type
 * @param {{ where: Object }} options
 * @param {StateTransitionExecutionContext} [StateTransitionExecutionContext]
 * @returns {Promise<Document[]>}
 */

/**
 * Create document
 *
 * @async
 * @method
 * @name StateRepository#createDocument
 * @param {Document} document
 * @param {StateTransitionExecutionContext} [StateTransitionExecutionContext]
 * @returns {Promise<void>}
 */

/**
 * Update document
 *
 * @async
 * @method
 * @name StateRepository#updateDocument
 * @param {Document} document
 * @param {StateTransitionExecutionContext} [StateTransitionExecutionContext]
 * @returns {Promise<void>}
 */

/**
 * Remove document
 *
 * @async
 * @method
 * @name StateRepository#removeDocument
 * @param {DataContract} dataContract
 * @param {string} type
 * @param {Identifier} id
 * @param {StateTransitionExecutionContext} [StateTransitionExecutionContext]
 * @returns {Promise<void>}
 */

/**
 * Fetch transaction by ID
 *
 * @async
 * @method
 * @name StateRepository#fetchTransaction
 * @param {string} id
 * @param {StateTransitionExecutionContext} [StateTransitionExecutionContext]
 * @returns {Promise<Object|null>}
 */

/**
 * Fetch identity by ID
 *
 * @async
 * @method
 * @name StateRepository#fetchIdentity
 * @param {Identifier} id
 * @param {StateTransitionExecutionContext} [StateTransitionExecutionContext]
 * @returns {Promise<Identity|null>}
 */

/**
 * Store identity
 *
 * @async
 * @method
 * @name StateRepository#createIdentity
 * @param {Identity} identity
 * @param {StateTransitionExecutionContext} [StateTransitionExecutionContext]
 * @returns {Promise<void>}
 */

/**
 * Store identity
 *
 * @async
 * @method
 * @name StateRepository#updateIdentity
 * @param {Identity} identity
 * @param {StateTransitionExecutionContext} [StateTransitionExecutionContext]
 * @returns {Promise<void>}
 */

/**
 * Store public keys hashes and identity id pair
 *
 * @async
 * @method
 * @name StateRepository#storeIdentityPublicKeyHashes
 * @param {Identifier} identityId
 * @param {Buffer[]} publicKeyHashes
 * @param {StateTransitionExecutionContext} [StateTransitionExecutionContext]
 * @returns {Promise<void>}
 */

/**
 * Fetch identity ids by public keys hashes
 *
 * @async
 * @method
 * @name StateRepository#fetchIdentityIdsByPublicKeyHashes
 * @param {Buffer[]} publicKeyHashes
 * @param {StateTransitionExecutionContext} [StateTransitionExecutionContext]
 * @returns {Promise<Array<Identifier|null>>}
 */

/**
 * Fetch latest platform block header
 *
 * @async
 * @method
 * @name StateRepository#fetchLatestPlatformBlockHeader
 * @returns {Promise<abci.IHeader>}
 */

/**
 * Verify Instant Lock
 *
 * @async
 * @method
 * @name StateRepository#verifyInstantLock
 * @param {InstantLock} instantLock
 * @param {StateTransitionExecutionContext} [StateTransitionExecutionContext]
 * @returns {Promise<boolean>}
 */

/**
 * Check if AssetLock Transaction outPoint exists in spent list
 *
 * @async
 * @method
 * @name StateRepository#isAssetLockTransactionOutPointAlreadyUsed
 * @param {Buffer} outPointBuffer
 * @param {StateTransitionExecutionContext} [StateTransitionExecutionContext]
 * @returns {Promise<boolean>}
 */

/**
 * Store AssetLock Transaction outPoint in spent list
 *
 * @async
 * @method
 * @name StateRepository#markAssetLockTransactionOutPointAsUsed
 * @param {Buffer} outPointBuffer
 * @param {StateTransitionExecutionContext} [StateTransitionExecutionContext]
 * @returns {Promise<void>}
 */

/**
 * Fetch Simplified Masternode List Store
 *
 * @async
 * @method
 * @name StateRepository#fetchSMLStore
 * @returns {Promise<SimplifiedMNListStore>}
 */
