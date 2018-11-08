describe('STPacketHeader', () => {
  describe('#setDapContractId', () => {
    it('should set Dap Contract ID');
  });
  describe('#getDapContractId', () => {
    it('should return Dap Contract ID');
  });
  describe('#setItemsMerkleRoot', () => {
    it('should set items merkle root');
  });
  describe('#getItemsMerkleRoot', () => {
    it('should get items merkle root');
  });
  describe('#setItemsHash', () => {
    it('should set items hash');
  });
  describe('#getItemsHash', () => {
    it('should get items hash');
  });
  describe('#toJSON', () => {
    it('should return ST Packet as plain object');
  });
  describe('#serialize', () => {
    it('should return serialized ST Packet');
  });
  describe('#hash', () => {
    it('should return ST Packet Header hash');
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
