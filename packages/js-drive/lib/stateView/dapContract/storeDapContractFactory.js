const StateTransitionPacket = require('../../../lib/storage/StateTransitionPacket');
const DapContract = require('./DapContract');

/**
 * @param {DapContractMongoDbRepository} dapContractRepository
 * @param {IpfsAPI} ipfs
 * @returns {storeDapContract}
 */
function storeDapContractFactory(dapContractRepository, ipfs) {
  /**
   * Validate and store DapContract
   *
   * @typedef {Promise} storeDapContract
   * @param {string} cid
   * @retuns {object}
   */
  return async function storeDapContract(cid) {
    const packetData = await ipfs.dag.get(cid);
    const packet = new StateTransitionPacket(packetData.value.data);
    const { dapid: dapId, objects: dapObjects, schema } = packet;
    const dapName = dapObjects[0].data.dapname;
    const dapContract = new DapContract(dapId, dapName, cid, schema);
    return dapContractRepository.store(dapContract);
  };
}

module.exports = storeDapContractFactory;
