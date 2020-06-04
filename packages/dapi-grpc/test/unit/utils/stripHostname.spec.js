const stripHostname = require('../../../lib/utils/stripHostname');

describe('stripHostname', () => {
  let hostname;

  beforeEach(() => {
    hostname = 'http://ip:3030/';
  });

  it('should strip everything and leave only hostname:port pair', () => {
    const result = stripHostname(hostname);

    expect(result).to.equal('ip:3030');
  });

  it('should strip everything and leave only ip:port pair', () => {
    hostname = 'http://127.0.0.1:3030/?some=params';

    const result = stripHostname(hostname);

    expect(result).to.equal('127.0.0.1:3030');
  });
});
