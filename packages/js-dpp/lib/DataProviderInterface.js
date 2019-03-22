/**
 * @classdesc DataProvider interface definition
 *
 * @async
 * @name DataProvider
 * @class
 */

/**
 * Fetch Contract by ID
 *
 * @async
 * @method
 * @name DataProvider#fetchContract
 * @param {string} id
 * @returns {Promise<Contract|null>}
 */

/**
 * Fetch Documents by contract ID and type
 *
 * @async
 * @method
 * @name DataProvider#fetchDocuments
 * @param {string} contractId
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
