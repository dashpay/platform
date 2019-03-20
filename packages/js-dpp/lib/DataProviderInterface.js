/**
 * @classdesc DataProvider interface definition
 *
 * @async
 * @name DataProvider
 * @class
 */

/**
 * Fetch DP Contract by ID
 *
 * @async
 * @method
 * @name DataProvider#fetchDPContract
 * @param {string} id
 * @returns {Promise<DPContract|null>}
 */

/**
 * Fetch Documents by contract ID and type
 *
 * @async
 * @method
 * @name DataProvider#fetchDocuments
 * @param {string} dpContractId
 * @param {string} type
 * @param {{ where: Object }} [options]
 * @returns {Promise<Document[]>}
 */

/**
 * Fetch transaction by ID
 *
 * @async
 * @method
 * @name DataProvider#fetchTransaction
 * @param {string} id
 * @returns {Promise<{ confirmations: number }|null>}
 */
