const PACKET_FIELDS = ['pver', 'dapid', 'dapobjectshash', 'dapcontract', 'dapobjects', 'meta'];

class StateTransitionPacket {
  constructor(data) {
    Object.assign(this, data);
  }

  /**
   * @param [skipMeta]
   */
  toJSON({ skipMeta = false }) {
    const result = {};
    PACKET_FIELDS.forEach((field) => {
      if (this[field] !== undefined) {
        result[field] = this[field];
      }
    });

    if (skipMeta) {
      delete result.meta;
    }

    return result;
  }
}

module.exports = StateTransitionPacket;
