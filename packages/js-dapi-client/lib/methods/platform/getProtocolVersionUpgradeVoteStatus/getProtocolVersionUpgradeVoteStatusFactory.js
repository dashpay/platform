const {
  v0: {
    PlatformPromiseClient,
    GetProtocolVersionUpgradeVoteStatusRequest,
  },
} = require('@dashevo/dapi-grpc');

const GetProtocolVersionUpgradeVoteStatusResponse = require('./GetProtocolVersionUpgradeVoteStatusResponse');
const InvalidResponseError = require('../response/errors/InvalidResponseError');

/**
 * @param {GrpcTransport} grpcTransport
 * @returns {getProtocolVersionUpgradeVoteStatus}
 */
function getProtocolVersionUpgradeVoteStatusFactory(grpcTransport) {
  /**
   * Fetch the version upgrade vote status
   * @typedef {getProtocolVersionUpgradeVoteStatus}
   * @param {string} startProTxHash
   * @param {number} count
   * @param {DAPIClientOptions & {prove: boolean}} [options]
   * @returns {Promise<GetProtocolVersionUpgradeVoteStatusResponse>}
   */
  async function getProtocolVersionUpgradeVoteStatus(startProTxHash, count, options = {}) {
    const {
      GetProtocolVersionUpgradeVoteStatusRequestV0,
    } = GetProtocolVersionUpgradeVoteStatusRequest;

    // eslint-disable-next-line max-len
    const getProtocolVersionUpgradeVoteStatusRequest = new GetProtocolVersionUpgradeVoteStatusRequest();

    getProtocolVersionUpgradeVoteStatusRequest.setV0(
      new GetProtocolVersionUpgradeVoteStatusRequestV0()
        .setStartProTxHash(Buffer.from(startProTxHash, 'hex'))
        .setCount(count)
        .setProve(!!options.prove),
    );

    let lastError;

    // TODO: simple retry before the dapi versioning is properly implemented
    for (let i = 0; i < 3; i += 1) {
      try {
        // eslint-disable-next-line no-await-in-loop
        const getProtocolVersionUpgradeVoteStatusResponse = await grpcTransport.request(
          PlatformPromiseClient,
          'getProtocolVersionUpgradeVoteStatus',
          getProtocolVersionUpgradeVoteStatusRequest,
          options,
        );

        return GetProtocolVersionUpgradeVoteStatusResponse
          .createFromProto(getProtocolVersionUpgradeVoteStatusResponse);
      } catch (e) {
        if (e instanceof InvalidResponseError) {
          lastError = e;
        } else {
          throw e;
        }
      }
    }

    // If we made it past the cycle it means that the retry didn't work,
    // and we're throwing the last error encountered
    throw lastError;
  }

  return getProtocolVersionUpgradeVoteStatus;
}

module.exports = getProtocolVersionUpgradeVoteStatusFactory;
