const chai = require('chai');
const chaiAsPromised = require('chai-as-promised');

chai.use(chaiAsPromised);
const { expect } = chai;

const assert = require('assert');
const Logger = require('../lib/log/Logger');

// TODO: Write unit tests
describe('Utils - Utils', () => {
  const logger = new Logger();
  const logger2 = new Logger('test.log');
  it('should be able to start a logger', () => {
    // TODO: test logger2 by checking file exist.
    assert.equal(typeof logger, 'object'); // bogus placeholder
    assert.equal(typeof logger2, 'object'); // bogus placeholder
  });

  it('should not be able to start a logger with invalid level', () => {
    expect(() => new Logger({ level: 'FAKE' })).to.throw('Logger: No log level matches FAKE');
  });
});
