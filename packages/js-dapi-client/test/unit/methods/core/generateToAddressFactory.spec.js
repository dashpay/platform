const generateToAddressFactory = require(
  '../../../../lib/methods/core/generateToAddressFactory',
);

describe('generateToAddressFactory', () => {
  let generateToAddress;
  let jsonRpcTransport;

  beforeEach(function beforeEach() {
    jsonRpcTransport = {
      request: this.sinon.stub(),
    };

    generateToAddress = generateToAddressFactory(jsonRpcTransport);
  });

  it('should call generateToAddress method', async () => {
    const resultData = 'result';
    const blocksNumber = 10;
    const address = 'yTMDce5yEpiPqmgPrPmTj7yAmQPJERUSVy';
    const options = {};
    jsonRpcTransport.request.resolves(resultData);

    const result = await generateToAddress(blocksNumber, address, options);
    expect(result).to.equal(resultData);
    expect(jsonRpcTransport.request).to.be.calledOnceWithExactly(
      'generateToAddress',
      { blocksNumber, address },
      options,
    );
  });
});
