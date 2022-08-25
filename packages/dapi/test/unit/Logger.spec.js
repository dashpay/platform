const assert = require('assert');
const Logger = require('../../lib/log/Logger');

describe('Logger', () => {
  it('should create a new Logger object', () => {
    const actual = typeof new Logger();
    const expected = 'object';
    assert.equal(actual, expected);
  });
  it('should default to the INFO log level', () => {
    const actual = new Logger().level;
    const expected = 4;
    assert.equal(actual, expected);
  });
  it('should default to logging to the console', () => {
    const actual = new Logger().outputFilePath;
    const expected = undefined;
    assert.equal(actual, expected);
  });
});
