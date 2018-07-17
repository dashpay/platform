const StateTransitionPacket = require('../../storage/StateTransitionPacket');
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
    const packet = new StateTransitionPacket(packetData.value);
    const { dapname: name, dapschema: schema } = packet.dapcontract;
    const dapContract = new DapContract(packet.dapid, name, cid, schema);
    return dapContractRepository.store(dapContract);
  };
}

module.exports = storeDapContractFactory;
