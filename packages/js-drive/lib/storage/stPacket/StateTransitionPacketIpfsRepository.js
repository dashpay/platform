const StateTransitionPacket = require('../stPacket/StateTransitionPacket');

const PinPacketTimeoutError = require('../errors/PinPacketTimeoutError');
const GetPacketTimeoutError = require('../errors/GetPacketTimeoutError');
const PacketNotPinnedError = require('../errors/PacketNotPinnedError');

const rejectAfter = require('../../util/rejectAfter');

class StateTransitionPacketIpfsRepository {
  /**
   * Create new instance of repository
   *
   * @param {IpfsApi} ipfsApi
   * @param {number} ipfsPinTimeout
   *
   * @return {StateTransitionPacketRepository}
   */
  constructor(ipfsApi, ipfsPinTimeout) {
    this.ipfsApi = ipfsApi;
    this.ipfsPinTimeout = ipfsPinTimeout;
  }

  /**
   * Find packets by CID
   *
   * @param {CID} cid
   *
   * @return {Promise<StateTransitionPacket>}
   */
  async find(cid) {
    const getPromise = this.ipfsApi.dag.get(cid);

    const error = new GetPacketTimeoutError();

    const { value: packetData } = await rejectAfter(getPromise, error, this.ipfsPinTimeout);

    return new StateTransitionPacket(packetData);
  }

  /**
   * Put a packet into IPFS
   *
   * @param {StateTransitionPacket} packet
   *
   * @return {Promise<CID>}
   */
  async store(packet) {
    const packetData = packet.toJSON({ skipMeta: true });
    return this.ipfsApi.dag.put(
      packetData,
      { cid: packet.getCID() },
    );
  }

  /**
   * Pin a packet by CID
   *
   * @param {CID} cid
   *
   * @return {Promise<void>}
   */
  async download(cid) {
    const storePromise = this.ipfsApi.pin.add(
      cid.toBaseEncodedString(),
      { recursive: true },
    );

    const error = new PinPacketTimeoutError();

    await rejectAfter(storePromise, error, this.ipfsPinTimeout);
  }

  /**
   * Unpin specific packet by CID
   *
   * @throws PacketNotPinnedError
   * @param {CID} cid
   *
   * @return {Promise<void>}
   */
  async delete(cid) {
    try {
      await this.ipfsApi.pin.rm(
        cid.toBaseEncodedString(),
        { recursive: true },
      );
    } catch (e) {
      if (e.message === 'not pinned') {
        throw new PacketNotPinnedError(cid);
      }
      throw e;
    }
  }

  /**
   * Unpin all recursive packets
   *
   * @return {Promise<void>}
   */
  async deleteAll() {
    const pinset = await this.ipfsApi.pin.ls();
    const byPinType = type => pin => pin.type === type;
    const pins = pinset.filter(byPinType('recursive'));

    for (let index = 0; index < pins.length; index++) {
      const pin = pins[index];
      await this.ipfsApi.pin.rm(pin.hash);
    }
  }
}

module.exports = StateTransitionPacketIpfsRepository;
