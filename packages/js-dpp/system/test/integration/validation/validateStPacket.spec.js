describe('validateStPacket', () => {
  it('should return error if packet is empty');
  it('should return error if packet doesn\'t contain `contractId`');
  it('should return error if packet doesn\'t contain `objects`');
  it('should return error if packet doesn\'t contain `contracts`');
  it('should return error if packet contains more than one contract');
  it('should return error if contract structure is wrong');
  it('should return error if object structure is wrong');
});
