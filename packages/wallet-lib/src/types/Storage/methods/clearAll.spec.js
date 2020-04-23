const { expect } = require('chai');
const clearState = require('./clearAll');

describe('Storage - clearAll', () => {
  it('should clear the whole state', () => {
    let called = 0;
    const self = {
      saveState: () => called++,
      store: {
        stuff: {},
        transactions: {
          nope: true,
        },
      },
    };
    clearState.call(self);
    expect(self.store).to.deep.equal({
      chains: {},
      transactions: {},
      wallets: {},
    });
    expect(called).to.equal(1);
  });
});
