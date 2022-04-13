const ValidatorNetworkInfo = require('../../../lib/validator/ValidatorNetworkInfo');

describe('ValidatorNetworkInfo', () => {
  let validatorNetworkInfo;
  let host;
  let port;

  beforeEach(() => {
    host = '192.168.65.2';
    port = 26656;
  });

  it('should return host', () => {
    validatorNetworkInfo = new ValidatorNetworkInfo(host, port);

    expect(validatorNetworkInfo.getHost()).to.equal(host);
  });

  it('should return port', () => {
    validatorNetworkInfo = new ValidatorNetworkInfo(host, port);

    expect(validatorNetworkInfo.getPort()).to.equal(port);
  });
});
