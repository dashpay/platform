const ValidatorNetworkInfo = require('./ValidatorNetworkInfo');

/**
 * @param {fetchProTxInfo} fetchProTxInfo
 * @param {number} p2pPort
 *
 * @return {fetchNetworkInfo}
 */
function fetchNetworkInfoFactory(fetchProTxInfo, p2pPort) {
  /**
   * @typedef fetchNetworkInfo
   *
   * @param {Object} quorumMember
   */
  async function fetchNetworkInfo(quorumMember) {
    const quorumMemberProTxInfo = await fetchProTxInfo(quorumMember.proTxHash);
    const quorumHost = quorumMemberProTxInfo.state.service.split(':')[0];

    return new ValidatorNetworkInfo(quorumHost, p2pPort);
  }

  return fetchNetworkInfo;
}

module.exports = fetchNetworkInfoFactory;
