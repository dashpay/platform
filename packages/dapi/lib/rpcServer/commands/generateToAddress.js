const Validator = require('../../utils/Validator');
const argsSchema = require('../schemas/generateToAddress.json');

const validator = new Validator(argsSchema);
/**
 * @param coreAPI
 * @return {generateToAddress}
 */
const generateToAddressFactory = (coreAPI) => {
  /**
   * Layer 1 endpoint
   * WORKS ONLY IN REGTEST MODE.
   * Generates blocks on demand for regression tests.
   * @typedef generateToAddress
   * @param args - command arguments
   * @param {number} args.blocksNumber - Number of blocks to generate
   * @param {string} args.address - The address that will receive the newly generated Dash
   *
   * @return {Promise<string[]>} - generated block hashes
   */
  async function generateToAddress(args) {
    validator.validate(args);

    const { blocksNumber, address } = args;

    return coreAPI.generateToAddress(blocksNumber, address);
  }

  return generateToAddress;
};

/* eslint-disable max-len */
/**
 * @swagger
 * /generateToAddress:
 *   post:
 *      operationId: generateToAddress
 *      deprecated: false
 *      summary: generate
 *      description: Generates blocks on demand sending funds to address
 *      tags:
 *        - L1
 *      responses:
 *        200:
 *          description: Successful response. Promise (string array) containing strings of block hashes.
 *      requestBody:
 *        content:
 *          application/json:
 *            schema:
 *              type: object
 *              required:
 *                - method
 *                - id
 *                - jsonrpc
 *                - params
 *              properties:
 *                method:
 *                  type: string
 *                  default: generate
 *                  description: Method name
 *                id:
 *                  type: integer
 *                  default: 1
 *                  format: int32
 *                  description: Request ID
 *                jsonrpc:
 *                  type: string
 *                  default: '2.0'
 *                  description: JSON-RPC Version (2.0)
 *                params:
 *                  title: Parameters
 *                  type: object
 *                  required:
 *                    - blocksNumber
 *                    - address
 *                  properties:
 *                    blocksNumber:
 *                      type: integer
 *                      default: 1
 *                      description: Number of blocks to generate
 *                    address:
 *                      type: string
 *                      description: Address to sends funds to
 */
/* eslint-enable max-len */

module.exports = generateToAddressFactory;
