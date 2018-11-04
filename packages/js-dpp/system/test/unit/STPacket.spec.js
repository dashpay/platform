describe('STPacket', () => {
  describe('#setDapContractId', () => {
    it('should set Dap Contract ID');
  });
  describe('#getDapContractId', () => {
    it('should return Dap Contract ID');
  });
  describe('#setDapContract', () => {
    it('should set Dap Contract');
    it('should throw error if Dap Objects are present');
  });
  describe('#getDapContract', () => {
    it('should return Dap Contract');
    it('should return null of Dap Contract is not present');
  });
  describe('#setDapObjects', () => {
    it('should set Dap Objects and replace previous');
    it('should throw error if Dap Contract is present');
  });
  describe('#getDapObjects', () => {
    it('should return Dap Objects');
  });
  describe('#addDapObject', () => {
    it('should add Dap Object');
    it('should add many Dap Objects');
  });
  describe('#toJSON', () => {
    it('should return ST Packet as plain object');
  });
  describe('#serialize', () => {
    it('should return serialized ST Packet');
  });
  describe('.fromObject', () => {
    it('should create ST Packet from plain object');
    it('should throw error if data is not valid');
  });
  describe('.fromSerialized', () => {
    it('should create ST Packet from string');
    it('should create ST Packet from buffer');
  });
  describe('.setSerializer', () => {
    it('should set serializer');
  });
  describe('.setStructureValidator', () => {
    it('should set structure validator');
  });
});
