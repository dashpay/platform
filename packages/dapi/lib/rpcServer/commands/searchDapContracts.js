const Validator = require('../../utils/Validator');
// TODO: Add name validation
const argsSchema = require('../schemas/searchDapContracts');

const validator = new Validator(argsSchema);

/**
 * @param dashDrive
 * @return {function({pattern: string}): Promise<Array<string>>}
 */
const searchDapContractsFactory = (dashDrive) => {
  /**
   * Layer 2 endpoint
   * Returns array of dap ids
   * @param args
   * @param {string} args.pattern
   * @return {Promise<Array<string>>}
   */
  async function searchDapContracts(args) {
    validator.validate(args);
    const { pattern } = args;
    return dashDrive.searchDapContracts(pattern);
  }

  return searchDapContracts;
};

module.exports = searchDapContractsFactory;
