const Validator = require('../../utils/Validator');
const argsSchema = require('../schemas/estimateFee');

const validator = new Validator(argsSchema);

/**
 * @param coreAPI
 * @return {estimateFee}
 */
const estimateFeeFactory = (coreAPI) => {
  /**
   * Layer 1 endpoint
   * estimates fee
   * @typedef estimateFee
   * @param {object} args
   * @param {number} args.nbBlocks - target
   * @return {Promise<number>} - fee in duffs per kilobyte
   */
  async function estimateFee(args) {
    validator.validate(args);
    const { nbBlocks } = args;
    return coreAPI.estimateFee(nbBlocks);
  }

  return estimateFee;
};

module.exports = estimateFeeFactory;
