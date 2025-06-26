import { Identifier } from '@dashevo/wasm-dpp';
import { expect } from 'chai';
import Fetcher from './index';

describe('Dash - Fetcher', () => {
  let fetcher: Fetcher;
  let dapiClientMock;
  const fetcherOptions = {
    delayMulMs: 0,
    maxAttempts: 3,
  };

  beforeEach(function beforeEach() {
    dapiClientMock = {
      platform: {
        getIdentity: this.sinon.stub(),
        getDataContract: this.sinon.stub(),
        getDocuments: this.sinon.stub(),
      },
    };

    fetcher = new Fetcher(dapiClientMock, fetcherOptions);
  });

  it('should acknowledge identifier', () => {
    const identifier = new Identifier(Buffer.alloc(32).fill(1));
    fetcher.acknowledgeIdentifier(identifier);
    expect(fetcher.hasIdentifier(identifier)).to.be.true;
  });

  it('should acknowledge string key', () => {
    fetcher.acknowledgeKey('key');
    expect(fetcher.hasKey('key')).to.be.true;
  });

  describe('fetchIdentity', () => {
    beforeEach(() => {
      dapiClientMock.platform.getIdentity.rejects();
    });

    it('should not re-try to fetch identity if it\'s identifier was not acknowledged', async () => {
      const identifier = new Identifier(Buffer.alloc(32).fill(1));
      await expect(fetcher.fetchIdentity(identifier)).to.be.rejected();
      expect(dapiClientMock.platform.getIdentity).to.be.calledOnce();
    });

    it('should re-try to fetch identity if it\'s identifier was acknowledged', async () => {
      const identifier = new Identifier(Buffer.alloc(32).fill(1));
      fetcher.acknowledgeIdentifier(identifier);
      await expect(fetcher.fetchIdentity(identifier)).to.be.rejected();
      expect(dapiClientMock.platform.getIdentity.callCount).to.be.equal(fetcherOptions.maxAttempts);
    });
  });

  describe('fetchDataContract', () => {
    beforeEach(() => {
      dapiClientMock.platform.getDataContract.rejects();
    });

    it('should not re-try to fetch data contract if it\'s identifier was not acknowledged', async () => {
      const identifier = new Identifier(Buffer.alloc(32).fill(1));
      await expect(fetcher.fetchDataContract(identifier)).to.be.rejected();
      expect(dapiClientMock.platform.getDataContract).to.be.calledOnce();
    });

    it('should re-try to fetch data contract if it\'s identifier was acknowledged', async () => {
      const identifier = new Identifier(Buffer.alloc(32).fill(1));
      fetcher.acknowledgeIdentifier(identifier);
      await expect(fetcher.fetchDataContract(identifier)).to.be.rejected();
      expect(dapiClientMock.platform.getDataContract.callCount)
        .to.be.equal(fetcherOptions.maxAttempts);
    });
  });

  describe('fetchDocuments', () => {
    beforeEach(() => {
      dapiClientMock.platform.getDocuments.resolves([]);
    });

    it('should not re-try to fetch documents if it\'s identifier was not acknowledged', async () => {
      const contractId = new Identifier(Buffer.alloc(32).fill(1));
      const type = 'niceDocument';
      await expect(fetcher.fetchDocuments(contractId, type, {})).to.be.rejected();
      expect(dapiClientMock.platform.getDocuments).to.be.calledOnce();
    });

    it('should re-try to fetch documents if it\'s identifier was acknowledged', async () => {
      const contractId = new Identifier(Buffer.alloc(32).fill(1));
      const type = 'niceDocument';
      const identifier = `${contractId.toString()}/${type}`;
      fetcher.acknowledgeKey(identifier);
      await expect(fetcher.fetchDocuments(contractId, type, {})).to.be.rejected();
      expect(dapiClientMock.platform.getDocuments.callCount)
        .to.be.equal(fetcherOptions.maxAttempts);
    });
  });
});
