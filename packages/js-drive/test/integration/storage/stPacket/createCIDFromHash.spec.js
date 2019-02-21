const CID = require('cids');
const multihashes = require('multihashes');

const createCIDFromHash = require('../../../../lib/storage/stPacket/createCIDFromHash');

const getSTPacketsFixture = require('../../../../lib/test/fixtures/getSTPacketsFixture');

const InvalidHashError = require('../../../../lib/storage/stPacket/errors/InvalidHashError');

describe('createCIDFromHash', () => {
  it('should create CID from a correct packet hash', async () => {
    const [stPacket] = getSTPacketsFixture();

    const cid = createCIDFromHash(stPacket.hash());

    expect(cid).to.be.an.instanceOf(CID);

    const { digest } = multihashes.decode(cid.multihash);

    expect(digest.toString('hex')).to.equal(stPacket.hash());
  });

  it('should throw InvalidHashError if packet hash is wrong', () => {
    let error;
    try {
      createCIDFromHash('wrong');
    } catch (e) {
      error = e;
    }

    expect(error).to.be.an.instanceOf(InvalidHashError);
  });
});
