describe('Sync Process', () => {
  it('should successfully finish initial sync emitting events, saving state and applying transitions');
  it('should skip errors in state transitions if option is set');
  it('should handle sequence validation by emitting events, throwing error and restarting reader');
  it('should halt syncing if unknown block error is thrown during reading');
});
