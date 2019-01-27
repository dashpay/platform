class PacketNotPinnedError extends Error {
  /**
   * @param {CID} cid
   */
  constructor(cid) {
    super();

    this.name = this.constructor.name;
    this.message = 'Packet is not pinned';
    this.cid = cid;

    Error.captureStackTrace(this, this.constructor);
  }

  /**
   * Return CID submitted with error
   *
   * @returns {CID}
   */
  getCID() {
    return this.cid;
  }
}

module.exports = PacketNotPinnedError;
