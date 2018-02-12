const { expect } = require('chai');

const addStateTransitionPacket = require('../../lib/storage/addStateTransitionPackets');

describe('Storage', () => {
  describe('addStateTransitionPackets', () => {
    it('should throws error', () => {
      expect(addStateTransitionPacket).to.throw('Not implemented yet');
    });
  });
});
