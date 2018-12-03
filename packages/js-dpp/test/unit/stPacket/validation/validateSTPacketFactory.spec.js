describe('validateSTPacketFactory', () => {
  it('should not validate items if structure is not valid');
  it('should validate structure and data');
  it('should validate contract if present');
  describe('ValidationResult', () => {
    it('should contain error if packet has invalid "contractId"');
    it('should contain error if packet has invalid "itemsHash"');
    it('should contain error if packet has invalid "itemsMerkleRoot"');
  });
});
