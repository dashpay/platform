const EventEmitter = require('events');
const cbor = require('cbor');
const networks = require('@dashevo/dashcore-lib/lib/networks');
const SimplifiedMNList = require('@dashevo/dashcore-lib/lib/deterministicmnlist/SimplifiedMNList');

const SimplifiedMasternodeListProvider = require('../../../lib/SimplifiedMasternodeListProvider/SimplifiedMasternodeListProvider');
const SimplifiedMasternodeListDAPIAddressProvider = require('../../../lib/dapiAddressProvider/SimplifiedMasternodeListDAPIAddressProvider');
const ListDAPIAddressProvider = require('../../../lib/dapiAddressProvider/ListDAPIAddressProvider');

// The full masternode list diff a testnet DAPI node streamed for block 1,582,244, after
// Core V24 activated at block 1,578,000. Its coinbase is a version 4 CbTx, and 12 of its
// 399 entries are version 3 (ExtAddr) entries, which Core hashes with ExtNetInfo.
const coreV24DiffFixture = require('../../fixtures/coreV24TestnetMasternodeListDiff.json');

describe('SimplifiedMasternodeListProvider with a Core V24 masternode list', () => {
  let diffBuffer;
  let regtestWasEnabled;
  let stream;
  let createStream;
  let smlProvider;

  before(async () => {
    // Synchronous cbor.encode silently truncates past its stream's buffer limit,
    // and the diff is far larger
    diffBuffer = await cbor.encodeAsync(coreV24DiffFixture);
  });

  beforeEach(function beforeEach() {
    // dashcore-lib's regtest is its shared testnet object with regtest switched on, and
    // looking up the 'local' or 'regtest' network switches it on for the whole process.
    // These cases need testnet itself.
    regtestWasEnabled = networks.testnet.regtestEnabled;
    networks.disableRegtest();

    stream = new EventEmitter();
    stream.cancel = this.sinon.stub();
    // The provider hands a diff it can't parse or apply to retryOnError. Turn that into
    // a stream error, so the test fails with the provider's error instead of a timeout
    // or an empty list.
    stream.retryOnError = this.sinon.stub().callsFake((error) => stream.emit('error', error));

    const response = {
      getMasternodeListDiff_asU8: () => new Uint8Array(diffBuffer),
    };

    createStream = this.sinon.stub().callsFake(async () => {
      // The provider subscribes to the stream after it resolves
      setTimeout(() => stream.emit('data', response), 0);

      return stream;
    });

    smlProvider = new SimplifiedMasternodeListProvider(createStream, {
      network: 'testnet',
      loggerOptions: {
        identifier: 'test',
      },
    });
  });

  afterEach(() => {
    if (regtestWasEnabled) {
      networks.enableRegtest();
    }
  });

  it('should rebuild the masternode list Core committed to in the coinbase', async () => {
    const sml = await smlProvider.getSimplifiedMNList();

    expect(sml).to.be.an.instanceOf(SimplifiedMNList);
    expect(sml.blockHash).to.equal(coreV24DiffFixture.blockHash);
    expect(sml.mnList).to.have.lengthOf(coreV24DiffFixture.mnList.length);
    expect(sml.mnList.filter((entry) => entry.nVersion === 3)).to.not.be.empty();

    const { extraPayload: cbTxPayload } = sml.cbTx;

    expect(cbTxPayload.version).to.equal(4);
    expect(sml.calculateMerkleRoot()).to.equal(cbTxPayload.merkleRootMNList);
    expect(sml.calculateMerkleRootQuorums()).to.equal(cbTxPayload.merkleRootQuorums);
  });

  it('should offer every valid evonode at the Platform HTTPS address Core reports', async () => {
    const listDAPIAddressProvider = new ListDAPIAddressProvider([], { network: 'testnet' });
    const dapiAddressProvider = new SimplifiedMasternodeListDAPIAddressProvider(
      smlProvider,
      listDAPIAddressProvider,
      [],
      { network: 'testnet' },
    );

    await dapiAddressProvider.getLiveAddress();

    const offeredAddresses = listDAPIAddressProvider.getAllAddresses()
      .map((address) => `${address.getHost()}:${address.getPort()}`);

    const platformHTTPSAddresses = coreV24DiffFixture.mnList
      .filter((entry) => entry.nType === 1 && entry.isValid)
      .map((entry) => entry.addresses.platform_https[0]);

    expect(platformHTTPSAddresses).to.not.be.empty();
    expect(offeredAddresses).to.have.members(platformHTTPSAddresses);
  });
});
