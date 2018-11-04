describe('DapObject', () => {
  describe('#setType', () => {
    it('should set $$type');
  });
  describe('#getType', () => {
    it('should return $$type');
  });
  describe('#setAction', () => {
    it('should set $$action');
  });
  describe('#getAction', () => {
    it('should return $$action');
  });
  describe('#toJSON', () => {
    it('should return Dap Object as plain object');
  });
  describe('#serialize', () => {
    it('should return serialized Dap Object');
  });
  describe('.fromObject', () => {
    it('should create Dap Object from plain object');
    it('should throw error if data is not valid');
  });
  describe('.fromSerialized', () => {
    it('should create Dap Object from string');
    it('should create Dap Object from buffer');
  });
  describe('.setSerializer', () => {
    it('should set serializer');
  });
  describe('.setStructureValidator', () => {
    it('should set structure validator');
  });
});
