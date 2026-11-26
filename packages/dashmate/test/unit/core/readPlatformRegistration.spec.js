import fs from 'fs';
import path from 'path';
import { fileURLToPath } from 'url';
import readPlatformRegistration, {
  REGISTRATION_STATUS,
  findPlatformRegistration,
  toLegacyBlsPublicKey,
} from '../../../src/core/readPlatformRegistration.js';

const fixturesDir = path.join(
  path.dirname(fileURLToPath(import.meta.url)),
  '..',
  '..',
  'fixtures',
  'coreRpc',
  '24.0.3',
);

/**
 * @param {string} name
 * @return {*}
 */
function loadFixture(name) {
  return JSON.parse(fs.readFileSync(path.join(fixturesDir, name), 'utf8'));
}

// Captured on a Dash Core 24.0.3 regtest node with v24 active: two Evo
// registrations (one with bare ports, one with ADDR:PORT addresses) and one
// Regular one. The operator keys are throwaway regtest keys.
const EVO = {
  operatorPrivateKey: '1bbac8e471b80050ea44afb514c87cb1ef03db5c66e643c7d6e0f09b29bedbc0',
  operatorPublicKey: 'a847dca38d98890c3c44e7d50bf4c654533b8c502169f406d7f2171b49494ed64998f2f555eced068ae3ce4551c476e9',
  proTxHash: 'd01f12613a758e772b5e319940a742fb6a659ad41c3f858fec3e94cf8447c707',
  platformNodeId: '29ecf89b23e4aa13d8b89bd286aa3cd1cb586cc4',
  externalIp: '1.2.3.4',
};

const EVO_WITH_ADDRESSES = {
  operatorPrivateKey: '0c9f8076444cabb3f319d3a349603cf5fdbb37ad02afa539083a2e45864a0a6c',
  proTxHash: 'e505e0d250c688f7287ad50743a22f5030a7dce71f31acdb9f390afd2840c186',
  externalIp: '1.2.3.6',
};

const REGULAR = {
  operatorPrivateKey: '72c8650e9a0d6c06b7792dd372f59f8746d88b155f84b16d795f50696c3bbf03',
  externalIp: '1.2.3.5',
};

const UNREGISTERED_OPERATOR_PRIVATE_KEY = '66e40ce736c2c0689e0372647d41a8c4cd14e6edf44cccff48068fee1cffe669';

describe('readPlatformRegistration', () => {
  let protxList;
  let mnsyncStatus;
  let blockchainInfo;
  let rpcClient;

  /**
   * @param {string} proTxHash
   * @return {Object} the list entry, to change for a derived case
   */
  function entry(proTxHash) {
    return protxList.find((item) => item.proTxHash === proTxHash);
  }

  /**
   * @param {Object} node
   * @return {Promise<Object>}
   */
  function read(node) {
    return readPlatformRegistration(rpcClient, {
      operatorPrivateKey: node.operatorPrivateKey,
      externalIp: node.externalIp,
    });
  }

  beforeEach(function beforeEach() {
    protxList = loadFixture('protx-list-registered-true.json');
    mnsyncStatus = loadFixture('mnsync-status-synced.json');
    blockchainInfo = loadFixture('getblockchaininfo.json');

    rpcClient = {
      mnsync: this.sinon.stub().callsFake(async () => ({ result: mnsyncStatus })),
      getBlockchainInfo: this.sinon.stub().callsFake(async () => ({ result: blockchainInfo })),
      callWithParams: this.sinon.stub().callsFake(async () => ({ result: protxList })),
    };
  });

  it('should find the Evo registration of this operator key with its identity and endpoints', async () => {
    const registration = await read(EVO);

    expect(registration).to.deep.equal({
      status: REGISTRATION_STATUS.EVO,
      proTxHash: EVO.proTxHash,
      platformNodeId: EVO.platformNodeId,
      platformP2PPort: 26656,
      platformHttpPort: 1443,
      poSeBanHeight: null,
    });
  });

  // dashd-rpc's `protx` spec turns every argument into a string, so `true`
  // would reach Core as "true".
  it('should ask Core for the detailed registered list with a real boolean', async () => {
    await read(EVO);

    expect(rpcClient.callWithParams).to.have.been.calledOnceWithExactly(
      'protx',
      ['list', 'registered', true],
    );
  });

  it('should read an Evo registration made with ADDR:PORT addresses', async () => {
    const registration = await read(EVO_WITH_ADDRESSES);

    expect(registration.status).to.equal(REGISTRATION_STATUS.EVO);
    expect(registration.proTxHash).to.equal(EVO_WITH_ADDRESSES.proTxHash);
    expect(registration.platformP2PPort).to.equal(26666);
    expect(registration.platformHttpPort).to.equal(1444);
  });

  it('should report a Regular registration', async () => {
    const registration = await read(REGULAR);

    expect(registration.status).to.equal(REGISTRATION_STATUS.REGULAR);
  });

  it('should report no registration for an operator key nobody registered', async () => {
    const registration = await read({
      operatorPrivateKey: UNREGISTERED_OPERATOR_PRIVATE_KEY,
      externalIp: EVO.externalIp,
    });

    expect(registration.status).to.equal(REGISTRATION_STATUS.NOT_FOUND);
  });

  // `masternode status` returns nothing for a banned node, which is why the
  // list is read instead. A banned node still has to be configured, so it can
  // be revived afterwards.
  it('should still find a PoSe-banned registration and report the ban', async () => {
    entry(EVO.proTxHash).state.PoSeBanHeight = 1310;

    const registration = await read(EVO);

    expect(registration.status).to.equal(REGISTRATION_STATUS.EVO);
    expect(registration.poSeBanHeight).to.equal(1310);
  });

  // Core reads the list at its own tip and never says it is behind, so an old
  // node ID would be taken for the current one.
  it('should not trust the list while Core is still syncing the blockchain', async () => {
    mnsyncStatus = loadFixture('mnsync-status-blockchain-syncing.json');

    const registration = await read(EVO);

    expect(registration.status).to.equal(REGISTRATION_STATUS.UNAVAILABLE);
    expect(registration.reason).to.match(/sync/);
    expect(rpcClient.callWithParams).to.not.have.been.called();
  });

  it('should not trust the list during initial block download', async () => {
    blockchainInfo = { ...blockchainInfo, initialblockdownload: true };

    const registration = await read(EVO);

    expect(registration.status).to.equal(REGISTRATION_STATUS.UNAVAILABLE);
  });

  it('should report an RPC failure as unavailable with its message', async () => {
    rpcClient.callWithParams.rejects(new Error('Work queue depth exceeded'));

    const registration = await read(EVO);

    expect(registration).to.deep.equal({
      status: REGISTRATION_STATUS.UNAVAILABLE,
      reason: 'Core RPC failed: Work queue depth exceeded',
    });
  });

  // Core before 23 has no `addresses` object, only the legacy fields.
  it('should read the endpoints from the legacy fields when there are no addresses', async () => {
    delete entry(EVO.proTxHash).state.addresses;

    const registration = await read(EVO);

    expect(registration.status).to.equal(REGISTRATION_STATUS.EVO);
    expect(registration.platformP2PPort).to.equal(26656);
    expect(registration.platformHttpPort).to.equal(1443);
  });

  // dashmate advertises Tenderdash at externalIp and issues the certificate for
  // it, so it cannot serve endpoints registered on another host.
  it('should refuse a registered Platform P2P address on another host', async () => {
    entry(EVO.proTxHash).state.addresses.platform_p2p = ['5.6.7.8:26656'];

    const registration = await read(EVO);

    expect(registration.status).to.equal(REGISTRATION_STATUS.UNSUPPORTED_ENDPOINTS);
    expect(registration.reason).to.include('5.6.7.8:26656').and.include(EVO.externalIp);
  });

  it('should refuse a registered HTTPS endpoint that is a domain name', async () => {
    entry(EVO.proTxHash).state.addresses.platform_https = ['evo.example.org:443'];

    const registration = await read(EVO);

    expect(registration.status).to.equal(REGISTRATION_STATUS.UNSUPPORTED_ENDPOINTS);
    expect(registration.reason).to.include('evo.example.org:443');
  });

  it('should refuse a registration whose Platform ports are unset', async () => {
    const { state } = entry(EVO.proTxHash);
    delete state.addresses;
    state.platformP2PPort = -1;

    const registration = await read(EVO);

    expect(registration.status).to.equal(REGISTRATION_STATUS.UNSUPPORTED_ENDPOINTS);
    expect(registration.reason).to.match(/P2P/);
  });

  // The external IP changed since registration: the node must be updated on
  // chain first, or Platform peers cannot reach it.
  it('should refuse when this node\'s external IP is not the registered one', async () => {
    const registration = await read({ ...EVO, externalIp: '9.9.9.9' });

    expect(registration.status).to.equal(REGISTRATION_STATUS.UNSUPPORTED_ENDPOINTS);
    expect(registration.reason).to.include('1.2.3.4:26656').and.include('9.9.9.9');
  });

  describe('findPlatformRegistration', () => {
    // Registrations made before v19 show the operator key in the legacy BLS
    // encoding, which differs from the basic one in its first byte.
    it('should match an operator key shown in the legacy encoding', () => {
      const legacy = toLegacyBlsPublicKey(EVO.operatorPublicKey);
      entry(EVO.proTxHash).state.pubKeyOperator = legacy;
      entry(EVO.proTxHash).state.version = 1;

      const registration = findPlatformRegistration(protxList, {
        operatorPublicKey: EVO.operatorPublicKey,
        externalIp: EVO.externalIp,
      });

      expect(legacy).to.not.equal(EVO.operatorPublicKey);
      expect(registration.status).to.equal(REGISTRATION_STATUS.EVO);
    });

    // Read as a basic key, the legacy encoding of ours is the key of its
    // negated point, which anyone can register without knowing its secret.
    it('should not take a basic registration of our key\'s legacy encoding for ours', () => {
      entry(EVO.proTxHash).state.pubKeyOperator = toLegacyBlsPublicKey(EVO.operatorPublicKey);

      const registration = findPlatformRegistration(protxList, {
        operatorPublicKey: EVO.operatorPublicKey,
        externalIp: EVO.externalIp,
      });

      expect(entry(EVO.proTxHash).state.version).to.equal(3);
      expect(registration.status).to.equal(REGISTRATION_STATUS.NOT_FOUND);
    });

    it('should not guess when two registrations claim the same operator key', () => {
      const duplicate = structuredClone(entry(EVO.proTxHash));
      duplicate.proTxHash = 'ff'.repeat(32);
      protxList.push(duplicate);

      const registration = findPlatformRegistration(protxList, {
        operatorPublicKey: EVO.operatorPublicKey,
        externalIp: EVO.externalIp,
      });

      expect(registration.status).to.equal(REGISTRATION_STATUS.UNAVAILABLE);
    });
  });

  describe('toLegacyBlsPublicKey', () => {
    // Pairs produced by Core's `bls fromsecret <secret>` and
    // `bls fromsecret <secret> true` for the same secrets.
    it('should give the encoding Core shows for legacy keys', () => {
      expect(toLegacyBlsPublicKey(EVO.operatorPublicKey))
        .to.equal('8847dca38d98890c3c44e7d50bf4c654533b8c502169f406d7f2171b49494ed64998f2f555eced068ae3ce4551c476e9');
      expect(toLegacyBlsPublicKey(`98${'00'.repeat(47)}`)).to.equal(`18${'00'.repeat(47)}`);
      expect(toLegacyBlsPublicKey(`80${'00'.repeat(47)}`)).to.equal(`00${'00'.repeat(47)}`);
    });
  });
});
