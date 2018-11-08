describe('verifyDapObjects', () => {
  it('should return invalid result if ST Packet contains duplicate Dap Objects');
  it('should return invalid result if Dap Object with action "create" is already present');
  it('should return invalid result if Dap Object with action "update" is not present');
  it('should return invalid result if Dap Object with action "delete" is not present');
  it('should return invalid result if Dap Object with action "update" has wrong revision');
  it('should return invalid result if Dap Object with action "delete" has wrong revision');
  it('should return invalid result if Dap Object has invalid action');
  it('should return valid result if Dap Objects are valid');
});
