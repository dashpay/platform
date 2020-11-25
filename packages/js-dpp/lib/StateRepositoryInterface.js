/**
 * @classdesc StateRepository interface definition
 *
 * @async
 * @name StateRepository
 * @class
 */

/**
 * Fetch Data Contract by ID
 *
 * @async
 * @method
 * @name StateRepository#fetchDataContract
 * @param {Identifier} id
 * @returns {Promise<DataContract|null>}
 */

/**
 * Store Data Contract
 *
 * @async
 * @method
 * @name StateRepository#storeDataContract
 * @param {DataContract} dataContract
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
 * @param {{ where: Object }} [options]
 * @returns {Promise<Document[]>}
 */

/**
 * Store document
 *
 * @async
 * @method
 * @name StateRepository#storeDocument
 * @param {Document} document
 * @returns {Promise<void>}
 */

/**
 * Remove document
 *
 * @async
 * @method
 * @name StateRepository#removeDocument
 * @param {Identifier} contractId
 * @param {string} type
 * @param {Identifier} id
 * @returns {Promise<void>}
 */

/**
 * Fetch transaction by ID
 *
 * @async
 * @method
 * @name StateRepository#fetchTransaction
 * @param {string} id
 * @returns {Promise<Object|null>}
 */

/**
 * Fetch identity by ID
 *
 * @async
 * @method
 * @name StateRepository#fetchIdentity
 * @param {Identifier} id
 * @returns {Promise<Identity|null>}
 */

/**
 * Store identity
 *
 * @async
 * @method
 * @name StateRepository#storeIdentity
 * @param {Identity} identity
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
 * @returns {Promise<void>}
 */

/**
 * Fetch identity ids by public keys hashes
 *
 * @async
 * @method
 * @name StateRepository#fetchIdentityIdsByPublicKeyHashes
 * @param {Buffer[]} publicKeyHashes
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
 * Fetch Simplified Masternode List Store
 *
 * @async
 * @method
 * @name StateRepository#fetchSMLStore
 * @returns {Promise<SimplifiedMNListStore>}
 */
