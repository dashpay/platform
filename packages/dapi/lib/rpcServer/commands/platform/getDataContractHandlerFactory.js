const RPCError = require('../../../rpcServer/RPCError');
const ValidationError = require('../../../errors/ArgumentsValidationError');

/**
 *
 * @param {DriveAdapter} driveAPI
 * @param {DashPlatformProtocol} dpp
 * @param {Validator} validator
 * @returns {getDataContractHandler}
 */
function getDataContractHandlerFactory(driveAPI, dpp, validator) {
  /**
   * @typedef getDataContractHandler
   * @param {Object} args
   * @param {string} args.id - contract id
   * @returns {Promise<{ dataContract: string }>}
   */
  async function getDataContractHandler(args) {
    validator.validate(args);
    const { id } = args;

    let dataContractJSON;

    try {
      dataContractJSON = await driveAPI.fetchContract(id);
    } catch (e) {
      if (e instanceof RPCError && e.code === -32602) {
        throw new ValidationError(e.message);
      }

      throw e;
    }

    const dataContract = await dpp.dataContract.createFromObject(
      dataContractJSON,
      { skipValidation: true },
    );

    return { dataContract: dataContract.serialize().toString('base64') };
  }

  return getDataContractHandler;
}

module.exports = getDataContractHandlerFactory;
