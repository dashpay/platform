/**
 * @classdesc DataProvider interface definition
 *
 * @async
 * @name DataProvider
 * @class
 */

/**
 * Fetch Data Contract by ID
 *
 * @async
 * @method
 * @name DataProvider#fetchDataContract
 * @param {string} id
 * @returns {Promise<DataContract|null>}
 */

/**
 * Fetch Documents by Data Contract ID and type
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
