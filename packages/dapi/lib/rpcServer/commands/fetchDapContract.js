const Validator = require('../../utils/Validator');
const argsSchema = require('../schemas/fetchDapContract');

const validator = new Validator(argsSchema);
/**
 * @param {AbstractDashDriveAdapter} dashDriveAPI
 * @return {fetchDapContract}
 */
const fetchDapContractFactory = (dashDriveAPI) => {
  /**
   * Layer 2 endpoint
   * Returns user dap space
   * @typedef fetchDapContract
   * @param args - command arguments
   * @param {string} args.dapId
   * @return {Promise<object>}
   */
  async function fetchDapContract(args) {
    validator.validate(args);
    const { dapId } = args;
    return dashDriveAPI.fetchDapContract(dapId);
  }

  return fetchDapContract;
};

module.exports = fetchDapContractFactory;
