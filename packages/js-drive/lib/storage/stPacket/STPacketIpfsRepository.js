const createCIDFromHash = require('./createCIDFromHash');

const PinPacketTimeoutError = require('../errors/PinPacketTimeoutError');
const GetPacketTimeoutError = require('../errors/GetPacketTimeoutError');
const PacketNotPinnedError = require('../errors/PacketNotPinnedError');

const rejectAfter = require('../../util/rejectAfter');

class STPacketIpfsRepository {
  /**
   * @param {IpfsAPI} ipfsApi
   * @param {DashPlatformProtocol} dpp
   * @param {number} ipfsPinTimeout
   */
  constructor(ipfsApi, dpp, ipfsPinTimeout) {
    this.ipfsApi = ipfsApi;
    this.dpp = dpp;
    this.ipfsPinTimeout = ipfsPinTimeout;
  }

  /**
   * Find packets by CID
   *
   * @param {CID} cid
   *
   * @return {Promise<STPacket>}
   */
  async find(cid) {
    const getPromise = this.ipfsApi.dag.get(cid);

    const error = new GetPacketTimeoutError();

    const { value: rawSTPacket } = await rejectAfter(getPromise, error, this.ipfsPinTimeout);

    return this.dpp.packet.createFromObject(rawSTPacket);
  }

  /**
   * Put a packet into IPFS
   *
   * @param {STPacket} stPacket
   *
   * @return {Promise<CID>}
   */
  async store(stPacket) {
    return this.ipfsApi.dag.put(
      stPacket.toJSON(),
      { cid: createCIDFromHash(stPacket.hash()) },
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

module.exports = STPacketIpfsRepository;
