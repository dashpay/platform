const { expect } = require('chai');

const { SPVError } = require('@dashevo/dash-spv');

const BlockHeadersProvider = require('../../../lib/BlockHeadersProvider/BlockHeadersProvider');
const getHeadersFixture = require('../../../lib/test/fixtures/getHeadersFixture');

describe('BlockHeadersProvider - unit', () => {
  let blockHeadersProvider;
  let headers;

  beforeEach(function () {
    blockHeadersProvider = new BlockHeadersProvider();
    blockHeadersProvider.setSpvChain({
      addHeaders: this.sinon.stub().callsFake((newHeaders) => newHeaders),
    });
    headers = getHeadersFixture();
    this.sinon.spy(blockHeadersProvider, 'emit');
  });

  describe('#handleHeaders', () => {
    it('should add headers to the spv chain and emit CHAIN_UPDATED event', () => {
      blockHeadersProvider.handleHeaders(headers, 1, () => {});
      expect(blockHeadersProvider.spvChain.addHeaders).to.have.been.calledWith(headers);
      expect(blockHeadersProvider.emit).to.have.been.calledWith('CHAIN_UPDATED', headers, 1);
    });

    it('should correctly calculate headHeight in case spv chain ignored some headers', () => {
      blockHeadersProvider.spvChain.addHeaders
        .callsFake((newHeaders) => newHeaders.slice(0, -1));
    });

    it('should not emit CHAIN_UPDATED in case spv chain ignored new headers', () => {
      blockHeadersProvider.spvChain.addHeaders.returns([]);
      blockHeadersProvider.handleHeaders(headers, 1, () => {});
      expect(blockHeadersProvider.emit).to.not.have.been.calledWith('CHAIN_UPDATED');
    });

    it('should reject headers in case of SPVError', () => {
      blockHeadersProvider.spvChain.addHeaders.throws(new SPVError('test'));
      blockHeadersProvider.handleHeaders(headers, 1, (err) => {
        expect(err).to.be.an.instanceOf(SPVError);
      });
    });

    it('should emit error in case of other errors', () => {
      const err = new Error('test');
      blockHeadersProvider.spvChain.addHeaders.throws(err);
      blockHeadersProvider.on('error', () => {});

      blockHeadersProvider.handleHeaders(headers, 1);
      expect(blockHeadersProvider.emit).to.have.been.calledWith('error', err);
    });
  });
});
