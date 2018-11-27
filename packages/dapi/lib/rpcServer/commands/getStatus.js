const Validator = require('../../utils/Validator');
const argsSchema = require('../schemas/getStatus');

const validator = new Validator(argsSchema);
/**
 * @param coreAPI
 * @return {getStatus}
 */
const getStatusFactory = (coreAPI) => {
  /**
   * Layer 1 endpoint
   * Returns calculated balance for the address
   * @typedef getStatus
   * @param args - command arguments
   * @param {string} args.query
   * @return {Promise<*>}
   */
  async function getStatus(args) {
    validator.validate(args);
    const { query } = args;
    return coreAPI.getStatus(query);
  }

  return getStatus;
};

module.exports = getStatusFactory;
