const Validator = require('../../utils/Validator');
const argsSchema = require('../schemas/generate');

const validator = new Validator(argsSchema);
/**
 * @param coreAPI
 * @return {generate}
 */
const generateFactory = (coreAPI) => {
  /**
   * WORKS ONLY IN REGTEST MODE.
   * Generates blocks on demand for regression tests.
   * @typedef generate
   * @param args - command arguments
   * @param {number} args.amount - amount of blocks to generate
   * @return {Promise<string[]>} - generated block hashes
   */
  async function generate(args) {
    validator.validate(args);
    const { amount } = args;
    return coreAPI.generate(amount);
  }

  return generate;
};

module.exports = generateFactory;
