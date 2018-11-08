describe('ObjectDataProvider', () => {
  describe('#constructor', () => {
    it('should populate Dap Contracts and DapObjects');
  });

  describe('#fetchDapContract', () => {
    it('should return Dap Contract by ID');
    it('should return null if Dap Contract is not present');
  });

  describe('#fetchDapObjects', () => {
    it('should return Dap Objects by primary key and type');
    it('should return empty array if such Dap Objects not found');
  });
});
