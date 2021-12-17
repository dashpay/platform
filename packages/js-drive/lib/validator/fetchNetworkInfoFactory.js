const ValidatorNetworkInfo = require('./ValidatorNetworkInfo');

/**
 * @param {fetchProTxInfo} fetchProTxInfo
 *
 * @return {fetchNetworkInfo}
 */
function fetchNetworkInfoFactory(fetchProTxInfo) {
  /**
   * @typedef fetchNetworkInfo
   *
   * @param {Object} quorumMember
   */
  async function fetchNetworkInfo(quorumMember) {
    const quorumMemberProTxInfo = await fetchProTxInfo(quorumMember.proTxHash);
    const quorumHost = quorumMemberProTxInfo.state.service.split(':')[0];

    return new ValidatorNetworkInfo(quorumHost, 26656);
  }

  return fetchNetworkInfo;
}

module.exports = fetchNetworkInfoFactory;
