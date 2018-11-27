/**
 * @param coreAPI
 * @return {getMNList}
 */
const getMNListFactory = (coreAPI) => {
  /**
   * Layer 1 endpoint
   * Returns masternode list
   * @typedef getMNList
   * @return {Promise<object[]>}
   */
  async function getMNList() {
    const insightMNList = await coreAPI.getMasternodesList();
    return insightMNList.map(masternode => Object.assign(masternode, { ip: masternode.ip.split(':')[0] }));
  }

  return getMNList;
};

module.exports = getMNListFactory;
