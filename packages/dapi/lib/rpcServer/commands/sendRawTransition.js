const { Transaction } = require('@dashevo/dashcore-lib');
const Schema = require('@dashevo/dash-schema/dash-schema-lib');
const crypto = require('crypto');
const Validator = require('../../utils/Validator');
const argsSchema = require('../schemas/sendRawTransition');

const validator = new Validator(argsSchema);

/**
 * Returns the string hex digest of a double SHA-256 hash.
 * @param {String|Buffer} data The incoming data to hash
 * @param {Object} cryptoLib The Node.js standard crypto library or equivalent
 * @return {String}
 */
const doubleSha256 = (data, cryptoLib = crypto) => {
  // The implementation of hash in Node.js is stateful and requires separate objects
  const hasher1 = cryptoLib.createHash('sha256');
  const firstHash = hasher1.update(data).digest();
  const hasher2 = cryptoLib.createHash('sha256');
  const secondHashHexDigest = hasher2.update(firstHash).digest('hex');
  return secondHashHexDigest;
};

const createStateTransition = ({
  TransactionClass = Transaction,
  rawTransitionHeader,
  rawTransitionDataPacket,
}) => {
  if (!rawTransitionDataPacket) {
    throw new Error('Updating state requires a transition data packet');
  }

  const rawTransitionDataPacketHexBuffer = Buffer.from(rawTransitionDataPacket, 'hex');
  const packet = Schema.serialize.decode(rawTransitionDataPacketHexBuffer);

  const packetHash = doubleSha256(rawTransitionDataPacketHexBuffer);
  // TODO: The following function is bugged and should be reported to Andy
  // const packetHash = SchemaClass.hash.stpacket(packet);
  const headerTransaction = new TransactionClass(rawTransitionHeader);
  const headerTransactionHash = headerTransaction.extraPayload.hashSTPacket;

  if (packetHash !== headerTransactionHash) {
    throw new Error('The hash of the data packet doesn\'t match the hash present in the header');
  }

  const stateTransition = {
    headerTransaction,
    packet,
  };

  return stateTransition;
};

/**
 * @param coreAPI
 * @param {AbstractDashDriveAdapter} dashDriveAPI
 * @return {function({rawTransitionHeader, rawTransitionPacket?}): string}
 */
function sendRawTransitionFactory(coreAPI, dashDriveAPI) {
  /**
   * Layer 2 endpoint
   * sends raw transition to quorum relay node
   * @typedef sendRawTransition
   * @param args
   * @param args.rawTransitionHeader
   * @param [args.rawTransitionPacket]
   * @return {Promise<string>}
   */
  const sendRawTransition = async (args) => {
    validator.validate(args);
    const { rawTransitionHeader, rawTransitionPacket: rawTransitionDataPacket } = args;
    const { headerTransaction: stateTransitionHeader } = createStateTransition({
      rawTransitionHeader,
      rawTransitionDataPacket,
    });
    await dashDriveAPI.addSTPacket(rawTransitionDataPacket);
    const txid = await coreAPI.sendRawTransaction(stateTransitionHeader.serialize());
    return txid;
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
 *                    - rawTransitionHeader
 *                  properties:
 *                    rawTransitionHeader:
 *                      type: string
 *                      default: ''
 *                      description: Raw transition
 */
/* eslint-enable max-len */

sendRawTransitionFactory.createStateTransition = createStateTransition;
sendRawTransitionFactory.doubleSha256 = doubleSha256;
module.exports = sendRawTransitionFactory;
