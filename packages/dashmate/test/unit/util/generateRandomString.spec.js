import generateRandomString from '../../../src/util/generateRandomString.js';

describe('generateRandomString', () => {
  it('should generate a 12-character alphanumeric password without Math.random', function it() {
    this.sinon.stub(Math, 'random').throws(new Error('Insecure randomness'));

    expect(generateRandomString(12)).to.match(/^[A-Za-z0-9]{12}$/);
  });
});
