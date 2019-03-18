const Validator = require('../../utils/Validator');
const argsSchema = require('../schemas/sendRawTransition');

const validator = new Validator(argsSchema);

function sendRawTransitionFactory(coreAPI, dashDriveAPI) {
  /**
   * Layer 2 endpoint
   * sends raw transition to quorum relay node and ST Packet to the local Drive
   *
   * @typedef sendRawTransition
   * @param args
   * @param args.rawStateTransition
   * @param args.rawSTPacket
   * @return {Promise<string>}
   */
  const sendRawTransition = async (args) => {
    validator.validate(args);

    const { rawStateTransition, rawSTPacket } = args;

    await dashDriveAPI.addSTPacket(rawStateTransition, rawSTPacket);

    return coreAPI.sendRawTransaction(rawStateTransition);
  };

  return sendRawTransition;
}

/* eslint-disable max-len */
/**
 * @swagger
 * /sendRawTransition:
 *   post:
 *      operationId: sendRawTransition
 *      deprecated: false
 *      summary: sendRawTransition
 *      description: Sends raw state transition to the network
 *      tags:
 *        - L2
 *      responses:
 *        200:
 *          description: Successful response. Promise (string) containing confirmed state transition transaction.
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
 *                  default: sendRawTransition
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
 *                    - rawStateTransition
 *                    - rawSTPacket
 *                  properties:
 *                    rawStateTransition:
 *                      type: string
 *                      default: ''
 *                      description: Raw transition
 *                    rawSTPacket:
 *                      type: string
 *                      default: ''
 *                      description: Raw ST Packet
 */
/* eslint-enable max-len */

module.exports = sendRawTransitionFactory;
