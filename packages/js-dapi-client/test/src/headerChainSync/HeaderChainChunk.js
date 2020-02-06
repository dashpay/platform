const HeaderChainChunk = require('../../../src/headerChainSync/HeaderChainChunk');

describe('HeaderChainChunk', () => {
  let headerChainChunk;
  let fromHeight;
  let size;
  let step;

  beforeEach(() => {
    fromHeight = 10;
    size = 10;
    step = 10;

    headerChainChunk = new HeaderChainChunk(
      fromHeight, size, step
    );
  });

  it('should return fromHeight', async () => {
    const result = headerChainChunk.getFromHeight();

    expect(result).to.equal(fromHeight);
  });

  it('should return size', async () => {
    const result = headerChainChunk.getSize();

    expect(result).to.equal(size);
  });

  it('should return step', async () => {
    const result = headerChainChunk.getStep();

    expect(result).to.equal(step);
  });

  it('should return toHeight', async () => {
    const result = headerChainChunk.getToHeight();

    expect(result).to.equal(fromHeight + size);
  });

  it('should return extra size', async () => {
    const result = headerChainChunk.getExtraSize();

    expect(result).to.equal(size % step);
  });
});
