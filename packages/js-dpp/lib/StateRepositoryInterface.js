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
 * @param {StateTransitionExecutionContext} [executionContext]
 * @returns {Promise<DataContract|null>}
 */

/**
 * Create Data Contract
 *
 * @async
 * @method
 * @name StateRepository#createDataContract
 * @param {DataContract} dataContract
 * @param {StateTransitionExecutionContext} [executionContext]
 * @returns {Promise<void>}
 */

/**
 * Update Data Contract
 *
 * @async
 * @method
 * @name StateRepository#updateDataContract
 * @param {DataContract} dataContract
 * @param {StateTransitionExecutionContext} [executionContext]
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
 * @param {StateTransitionExecutionContext} [executionContext]
 * @returns {Promise<Document[]>}
 */

/**
 * Create document
 *
 * @async
 * @method
 * @name StateRepository#createDocument
 * @param {Document} document
 * @param {StateTransitionExecutionContext} [executionContext]
 * @returns {Promise<void>}
 */

/**
 * Update document
 *
 * @async
 * @method
 * @name StateRepository#updateDocument
 * @param {Document} document
 * @param {StateTransitionExecutionContext} [executionContext]
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
 * @param {StateTransitionExecutionContext} [executionContext]
 * @returns {Promise<void>}
 */

/**
 * Fetch transaction by ID
 *
 * @async
 * @method
 * @name StateRepository#fetchTransaction
 * @param {string} id
 * @param {StateTransitionExecutionContext} [executionContext]
 * @returns {Promise<Object|null>}
 */

/**
 * Fetch identity by ID
 *
 * @async
 * @method
 * @name StateRepository#fetchIdentity
 * @param {Identifier} id
 * @param {StateTransitionExecutionContext} [executionContext]
 * @returns {Promise<Identity|null>}
 */

/**
 * Create identity
 *
 * @async
 * @method
 * @name StateRepository#createIdentity
 * @param {Identity} identity
 * @param {StateTransitionExecutionContext} [executionContext]
 * @returns {Promise<void>}
 */

/**
 * Add keys to identity
 *
 * @async
 * @method
 * @name StateRepository#addKeysToIdentity
 * @param {Identifier} identityId
 * @param {IdentityPublicKey[]} keys
 * @param {StateTransitionExecutionContext} [executionContext]
 * @returns {Promise<void>}
 */

/**
 * Disable identity keys
 *
 * @async
 * @method
 * @name StateRepository#disableIdentityKeys
 * @param {Identifier} identityId
 * @param {number[]} keyIds
 * @param {number} disableAt
 * @param {StateTransitionExecutionContext} [executionContext]
 * @returns {Promise<void>}
 */

/**
 * Update identity revision
 *
 * @async
 * @method
 * @name StateRepository#updateIdentityRevision
 * @param {Identifier} identityId
 * @param {number} revision
 * @param {StateTransitionExecutionContext} [executionContext]
 * @returns {Promise<void>}
 */

/**
 * Add to identity balance
 *
 * @async
 * @method
 * @name StateRepository#addToIdentityBalance
 * @param {Identifier} identityId
 * @param {number} amount
 * @param {StateTransitionExecutionContext} [executionContext]
 * @returns {Promise<void>}
 */

/**
 * Add to system credits
 *
 * @async
 * @method
 * @name StateRepository#addToSystemCredits
 * @param {number} amount
 * @param {StateTransitionExecutionContext} [executionContext]
 * @returns {Promise<void>}
 */

/**
 * Fetch the latest platform block time
 *
 * @async
 * @method
 * @name StateRepository#fetchLatestPlatformBlockTime
 * @returns {Promise<protobuf.Timestamp>}
 */

/**
 * Fetch the latest platform block height
 *
 * @async
 * @method
 * @name StateRepository#fetchLatestPlatformBlockHeight
 * @returns {Promise<Long>}
 */

/**
 * Fetch the latest platform core chainlocked height
 *
 * @async
 * @method
 * @name StateRepository#fetchLatestPlatformCoreChainLockedHeight
 * @returns {Promise<number>}
 */

/**
 * Verify Instant Lock
 *
 * @async
 * @method
 * @name StateRepository#verifyInstantLock
 * @param {InstantLock} instantLock
 * @param {StateTransitionExecutionContext} [executionContext]
 * @returns {Promise<boolean>}
 */

/**
 * Check if AssetLock Transaction outPoint exists in spent list
 *
 * @async
 * @method
 * @name StateRepository#isAssetLockTransactionOutPointAlreadyUsed
 * @param {Buffer} outPointBuffer
 * @param {StateTransitionExecutionContext} [executionContext]
 * @returns {Promise<boolean>}
 */

/**
 * Store AssetLock Transaction outPoint in spent list
 *
 * @async
 * @method
 * @name StateRepository#markAssetLockTransactionOutPointAsUsed
 * @param {Buffer} outPointBuffer
 * @param {StateTransitionExecutionContext} [executionContext]
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

/**
 * Returns current block time in milliseconds
 *
 * @async
 * @method
 * @name StateRepository#fetchLatestPlatformBlockTime
 * @returns {Promise<number>}
 */
