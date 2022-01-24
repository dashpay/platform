const BlockHeadersProvider = require('../../../lib/BlockHeadersProvider/BlockHeadersProvider');
const createBlockHeadersProviderFromOptions = require('../../../lib/BlockHeadersProvider/createBlockHeadersProviderFromOptions');

describe('#createBlockHeadersProviderFromOptions', () => {
  const coreMethodsMock = {};
  let options;

  beforeEach(() => {
    options = {
      network: 'testnet',
    };
  });

  it('should create BlockHeadersProvider with default options', () => {
    const provider = createBlockHeadersProviderFromOptions(options, coreMethodsMock);
    expect(provider).to.be.instanceOf(BlockHeadersProvider);
  });

  it('should create BlockHeadersProvider from \'blockHeadersProvider\' option', () => {
    const blockHeadersProvider = new BlockHeadersProvider();
    options.blockHeadersProvider = blockHeadersProvider;
    const provider = createBlockHeadersProviderFromOptions(options, coreMethodsMock);
    expect(provider).to.equal(blockHeadersProvider);
  });

  it('should create BlockHeadersProvider from \'blockHeadersProviderOptions\' option', () => {
    options = {
      ...options,
      blockHeadersProviderOptions: {
        maxRetries: 100,
        maxParallelStreams: 5,
        targetBatchSize: 10,
        fromBlockHeight: 5,
      },
    };

    const provider = createBlockHeadersProviderFromOptions(options, coreMethodsMock);
    Object.keys(options.blockHeadersProviderOptions).forEach((key) => {
      expect(provider.options[key]).to.equal(options.blockHeadersProviderOptions[key]);
    });
  });

  it('should validate options', () => {
    const badOptions = {
      maxRetries: -1,
      maxParallelStreams: 0,
      targetBatchSize: 0,
      fromBlockHeight: 0,
      autoStart: 'true',
      network: 'keknet',
    };

    Object.keys(badOptions).forEach((badOption) => {
      options = {
        ...options,
        blockHeadersProviderOptions: {
          [badOption]: badOptions[badOption],
        },
      };
      expect(() => createBlockHeadersProviderFromOptions(options, coreMethodsMock)).to.throw();
    });
  });
});
