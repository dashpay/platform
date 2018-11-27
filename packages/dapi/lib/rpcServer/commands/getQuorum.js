const Validator = require('../../utils/Validator');
const argsSchema = require('../schemas/getQuorum');

const validator = new Validator(argsSchema);
/**
 * @param coreApi
 * @return {getQuorum}
 */
const getQuorumFactory = (coreApi) => {
  /**
   * Returns user quorum (llmq)
   * @typedef getQuorum
   * @param args - command arguments
   * @param {string} args.regTxId
   * @return {Promise<object>}
   */
  async function getQuorum(args) {
    validator.validate(args);
    const { regTxId } = args;
    return coreApi.getQuorum(regTxId);
  }

  return getQuorum;
};

module.exports = getQuorumFactory;
