const {
  v0: {
    PlatformPromiseClient,
    GetVersionUpgradeVoteStatusRequest,
  },
} = require('@dashevo/dapi-grpc');

const GetVersionUpgradeVoteStatusResponse = require('./GetVersionUpgradeVoteStatusResponse');
const InvalidResponseError = require('../response/errors/InvalidResponseError');

/**
 * @param {GrpcTransport} grpcTransport
 * @returns {getVersionUpgradeVoteStatus}
 */
function getVersionUpgradeVoteStatusFactory(grpcTransport) {
  /**
   * Fetch the version upgrade vote status
   *
   * @typedef {getVersionUpgradeVoteStatus}
   * @param {string} startProTxHash
   * @param {number} count
   * @param {DAPIClientOptions & {prove: boolean}} [options]
   * @returns {Promise<GetVersionUpgradeVoteStatusResponse>}
   */
  async function getVersionUpgradeVoteStatus(startProTxHash, count, options = {}) {
    const { GetVersionUpgradeVoteStatusRequestV0 } = GetVersionUpgradeVoteStatusRequest;
    const getVersionUpgradeVoteStatusRequest = new GetVersionUpgradeVoteStatusRequest();
    getVersionUpgradeVoteStatusRequest.setV0(
      new GetVersionUpgradeVoteStatusRequestV0()
        .setStartProTxHash(Buffer.from(startProTxHash, 'hex'))
        .setCount(count)
        .setProve(!!options.prove),
    );

    let lastError;

    // TODO: simple retry before the dapi versioning is properly implemented
    for (let i = 0; i < 3; i += 1) {
      try {
        // eslint-disable-next-line no-await-in-loop
        const getVersionUpgradeVoteStatusResponse = await grpcTransport.request(
          PlatformPromiseClient,
          'getVersionUpgradeVoteStatus',
          getVersionUpgradeVoteStatusRequest,
          options,
        );

        return GetVersionUpgradeVoteStatusResponse
          .createFromProto(getVersionUpgradeVoteStatusResponse);
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

  return getVersionUpgradeVoteStatus;
}

module.exports = getVersionUpgradeVoteStatusFactory;
