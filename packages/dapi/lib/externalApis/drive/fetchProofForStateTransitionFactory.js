const {
  v0: {
    GetProofsRequest,
  },
} = require('@dashevo/dapi-grpc');

/**
 * @param {PlatformPromiseClient} driveClient
 * @return {fetchProofForStateTransition}
 */
function fetchProofForStateTransitionFactory(driveClient) {
  /**
   * @typedef {fetchProofForStateTransition}
   * @param {Uint8Array} stateTransition
   * @return {Promise<GetProofsResponse>}
   */
  async function fetchProofForStateTransition(stateTransition) {
    const request = new GetProofsRequest();

    request.setStateTransition(stateTransition);

    return driveClient.getProofs(request);
  }

  return fetchProofForStateTransition;
}

module.exports = fetchProofForStateTransitionFactory;
