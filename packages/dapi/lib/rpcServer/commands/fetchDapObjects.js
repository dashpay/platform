const Validator = require('../../utils/Validator');
const argsSchema = require('../schemas/fetchDapObjects');

const validator = new Validator(argsSchema);
/**
 * @param {AbstractDashDriveAdapter} dashDriveAPI
 * @return {fetchDapObjects}
 */
const fetchDapObjectsFactory = (dashDriveAPI) => {
  /**
   * Fetches user objects for a given condition
   * @typedef fetchDapObjects
   * @param args - command arguments
   * @param {string} args.dapId
   * @param {string} args.type
   * @param args.options
   * @param {Object} args.options.where - Mongo-like query
   * @param {Object} args.options.orderBy - Mongo-like sort field
   * @param {number} args.options.limit - how many objects to fetch
   * @param {number} args.options.startAt - number of objects to skip
   * @param {number} args.options.startAfter - exclusive skip
   * @return {Promise<object>}
   */
  async function fetchDapObjects(args) {
    validator.validate(args);
    const { dapId, type, options } = args;
    return dashDriveAPI.fetchDapObjects(dapId, type, options);
  }

  return fetchDapObjects;
};

module.exports = fetchDapObjectsFactory;
