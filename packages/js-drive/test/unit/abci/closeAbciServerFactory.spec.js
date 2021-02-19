const closeAbciServerFactory = require('../../../lib/abci/closeAbciServerFactory');

describe('closeAbciServerFactory', () => {
  let closeAbciServer;
  let abciServerMock;

  beforeEach(function beforeEach() {
    abciServerMock = {
      close: this.sinon.spy((resolve) => {
        resolve();
      }),
      listening: true,
    };

    closeAbciServer = closeAbciServerFactory(abciServerMock);
  });

  it('should close server if it\'s listening', async () => {
    await closeAbciServer();

    expect(abciServerMock.close).to.be.calledOnce();
  });

  it('should not close server if not listening', async () => {
    abciServerMock.listening = false;

    expect(abciServerMock.close).to.not.be.called();
  });
});
