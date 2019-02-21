const bypassFactory = require('../../../../lib/api/middlewares/bypassFactory');

describe('bypassFactory', () => {
  let next;
  let method;

  beforeEach(function beforeEach() {
    next = this.sinon.spy();
    method = this.sinon.spy();
  });

  it('should bypass method call, calling next(), if request method is in the whitelist', () => {
    const req = {
      body: {
        method: 'getSyncInfo',
      },
    };

    const whitelist = ['getSyncInfo'];
    const bypass = bypassFactory(method, whitelist);
    bypass(req, null, next);

    expect(next).to.have.been.calledOnce();
    expect(method).to.have.not.been.calledOnce();
  });

  it('should not bypass method call if request method is not in the whitelist', () => {
    const req = {
      body: {
        method: 'fetchDPObjects',
      },
    };

    const whitelist = ['getSyncInfo'];
    const bypass = bypassFactory(method, whitelist);
    bypass(req, null, next);

    expect(next).to.have.not.been.calledOnce();
    expect(method).to.have.been.calledOnce();
  });
});
