import init, * as sdk from '../../dist/sdk.compressed.js';
import { prefetchLocalReady } from './helpers/trustedContext.ts';
import { wasmFunctionalTestRequirements } from './fixtures/requiredTestData.ts';

describe('Voting', function describeVoting() {
  this.timeout(60000);

  let client: sdk.WasmSdk;
  let builder: sdk.WasmSdkBuilder;
  const { dpnsContractId, dpnsDomain, identityId } = wasmFunctionalTestRequirements();

  before(async () => {
    await init();
    const context = await prefetchLocalReady();
    builder = sdk.WasmSdkBuilder.local().withTrustedContext(context);
    client = await builder.build();
  });

  after(() => {
    if (client) {
      client.free();
    }
  });

  describe('getContestedResources()', () => {
    it('should list contested resources', async () => {
      const DPNS_CONTRACT = dpnsContractId;

      await client.getContestedResources({
        dataContractId: DPNS_CONTRACT,
        documentTypeName: 'domain',
        indexName: 'parentNameAndLabel',
        orderAscending: true,
      });
    });
  });

  describe('getContestedResourceVoteState()', () => {
    it('should get contested resource vote state', async () => {
      const DPNS_CONTRACT = dpnsContractId;
      const PARENT = dpnsDomain.parent;
      const LABEL = dpnsDomain.label;

      await client.getContestedResourceVoteState({
        dataContractId: DPNS_CONTRACT,
        documentTypeName: 'domain',
        indexName: 'parentNameAndLabel',
        indexValues: [PARENT, LABEL],
        resultType: 'documents',
        limit: 50,
        includeLockedAndAbstaining: true,
      });
    });
  });

  describe('getContestFundToJoin()', () => {
    function domain(label: string, normalizedLabel: string): sdk.Document {
      return new sdk.Document({
        properties: {
          label,
          normalizedLabel,
          parentDomainName: 'dash',
          normalizedParentDomainName: 'dash',
          preorderSalt: new Uint8Array(32),
          records: { identity: new Uint8Array(32) },
          subdomainRules: { allowSubdomains: false },
        },
        documentTypeName: 'domain',
        dataContractId: dpnsContractId,
        ownerId: identityId,
      });
    }

    it('should state the contest fund on the contested index for a contested name', async () => {
      const balance = await client.getContestFundToJoin(domain('quantum', 'quantum'));

      expect(balance).to.be.an.instanceOf(sdk.PrefundedVotingBalance);
      expect(balance?.indexName).to.equal('parentNameAndLabel');
      // The DPNS contest fund at the network's version, until a contest holds 250 contenders
      const contract = await client.getDataContract(dpnsContractId);
      if (!contract) {
        throw new Error('expected the DPNS contract');
      }
      const [contestFund] = sdk.documentCreateCost(contract, 'domain', undefined, client.version())
        .contractCharges.flatMap((charge) => (charge.kind === 'contestFund' ? [charge.credits] : []));
      expect(balance?.credits).to.equal(BigInt(contestFund));
    });

    it('should resolve to undefined for a name that is not contested', async () => {
      const label = 'contest-fund-test-name-2';
      const balance = await client.getContestFundToJoin(domain(label, label));

      expect(balance).to.be.undefined();
    });
  });

  describe('getVotePollsByEndDate()', () => {
    const DAY_MS = 24 * 60 * 60 * 1000;

    it('should accept millisecond timestamps given as numbers', async () => {
      const now = Date.now();

      const entries = await client.getVotePollsByEndDate({
        startTimeMs: now - 30 * DAY_MS,
        endTimeMs: now + 30 * DAY_MS,
        limit: 10,
      });

      expect(entries).to.be.an('array');
    });

    it('should accept millisecond timestamps given as bigints', async () => {
      const entries = await client.getVotePollsByEndDate({
        startTimeMs: BigInt(Date.now()),
        startTimeIncluded: false,
        limit: 10,
      });

      expect(entries).to.be.an('array');
    });
  });
});
