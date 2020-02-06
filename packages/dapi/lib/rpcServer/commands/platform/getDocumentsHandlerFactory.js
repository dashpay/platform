const ArgumentsValidationError = require('../../../errors/ArgumentsValidationError');

const RPCError = require('../../../rpcServer/RPCError');
/**
 *
 * @param {DriveAdapter} driveAPI
 * @param {DashPlatformProtocol} dpp
 * @param {Validator} validator
 * @returns {getDocumentsHandler}
 */
function getDocumentsHandlerFactory(driveAPI, dpp, validator) {
  /**
   * @typedef getDocumentsHandler
   * @param {Object} args
   * @param {Object} args.contractId
   * @param {string} args.documentType
   * @param {Object} args.where
   * @param {Object} args.orderBy
   * @param {number} args.limit
   * @param {number} args.startAfter
   * @param {number} args.startAt
   * @returns {Promise<GetDocumentsResponse>}
   */
  async function getDocumentsHandler(args) {
    validator.validate(args);
    const {
      dataContractId, documentType, where, orderBy, startAt, limit, startAfter,
    } = args;

    if (!dataContractId) {
      throw new ArgumentsValidationError('dataContractId is not specified');
    }

    if (!documentType) {
      throw new ArgumentsValidationError('documentType is not specified');
    }

    const options = {
      where,
      orderBy,
      limit,
      startAfter,
      startAt,
    };

    let documentsJSON;
    try {
      documentsJSON = await driveAPI.fetchDocuments(dataContractId, documentType, options);
    } catch (e) {
      if (e instanceof RPCError && e.code === -32602) {
        throw new ArgumentsValidationError(e.message, null, e.data);
      }

      throw e;
    }

    return Promise.all(
      documentsJSON.map(documentJSON => dpp.document.createFromObject(
        documentJSON,
        { skipValidation: true },
      )),
    );
  }

  return getDocumentsHandler;
}

module.exports = getDocumentsHandlerFactory;
