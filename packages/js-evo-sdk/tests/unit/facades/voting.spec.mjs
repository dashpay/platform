import init, * as wasmSDKPackage from '@dashevo/wasm-sdk';
import { EvoSDK } from '../../../dist/sdk.js';

describe('VotingFacade', () => {
  let wasmSdk;
  let client;
  let signer;

  // Realistic identifiers
  const dataContractId = 'GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec';
  const identityId = '5mjGWa9mruHnLBht3ntBi8CZ6sNk3hZZsQMgTvgQobjS';
  const contenderId = '6o4vL6YpPjamqnnPNpwNSspYJdhPpzYbXvAJ4PYH7Ack';
  const masternodeProTxHash = 'a1b2c3d4e5f6a7b8c9d0e1f2a3b4c5d6e7f8a9b0c1d2e3f4a5b6c7d8e9f0a1b2';

  beforeEach(async function setup() {
    await init();
    const builder = wasmSDKPackage.WasmSdkBuilder.testnetTrusted();
    wasmSdk = builder.build();
    client = EvoSDK.fromWasm(wasmSdk);

    // Create mock objects
    signer = Object.create(wasmSDKPackage.IdentitySigner.prototype);

    // Stub query methods
    this.sinon.stub(wasmSdk, 'getContestedResourceVoteState').resolves({
      contenders: new Map(),
      abstainVoteCount: 0,
      lockVoteCount: 0,
    });
    this.sinon.stub(wasmSdk, 'getContestedResourceVoteStateWithProofInfo').resolves({
      data: {
        contenders: new Map(),
        abstainVoteCount: 0,
        lockVoteCount: 0,
      },
      proof: {},
      metadata: {},
    });
    this.sinon.stub(wasmSdk, 'getContestedResourceIdentityVotes').resolves([]);
    this.sinon.stub(wasmSdk, 'getContestedResourceIdentityVotesWithProofInfo').resolves({
      data: [],
      proof: {},
      metadata: {},
    });
    this.sinon.stub(wasmSdk, 'getVotePollsByEndDate').resolves([]);
    this.sinon.stub(wasmSdk, 'getVotePollsByEndDateWithProofInfo').resolves({
      data: [],
      proof: {},
      metadata: {},
    });

    // Stub transition method
    this.sinon.stub(wasmSdk, 'masternodeVote').resolves({
      success: true,
    });
  });

  describe('Query Methods', () => {
    it('contestedResourceVoteState() fetches vote state for a contested resource', async () => {
      const query = {
        dataContractId,
        documentTypeName: 'domain',
        indexName: 'parentNameAndLabel',
        indexValues: ['dash', 'alice'],
        resultType: 'documentsAndVoteTally',
      };

      await client.voting.contestedResourceVoteState(query);

      expect(wasmSdk.getContestedResourceVoteState).to.be.calledOnceWithExactly(query);
    });

    it('contestedResourceVoteStateWithProof() fetches vote state with proof and pagination', async () => {
      const query = {
        dataContractId,
        documentTypeName: 'domain',
        indexName: 'parentNameAndLabel',
        indexValues: ['dash', 'bob'],
        resultType: 'documentsAndVoteTally',
        includeLockedAndAbstaining: true,
        startAtContenderId: contenderId,
        startAtIncluded: true,
        limit: 10,
      };

      await client.voting.contestedResourceVoteStateWithProof(query);

      expect(wasmSdk.getContestedResourceVoteStateWithProofInfo).to.be.calledOnceWithExactly(query);
    });

    it('contestedResourceIdentityVotes() fetches votes cast by an identity', async () => {
      const query = {
        identityId,
        limit: 50,
        startAtVoteId: 'AYjR8WFgxoQqeVqKhFNxQvPsaJSZPvzqDJdw4YrFBEhT',
        orderAscending: true,
      };

      await client.voting.contestedResourceIdentityVotes(query);

      expect(wasmSdk.getContestedResourceIdentityVotes).to.be.calledOnceWithExactly(query);
    });

    it('contestedResourceIdentityVotesWithProof() fetches identity votes with proof', async () => {
      const query = {
        identityId,
        limit: 25,
        startAtVoteId: 'BYjR8WFgxoQqeVqKhFNxQvPsaJSZPvzqDJdw4YrFBEhT',
        orderAscending: false,
      };

      await client.voting.contestedResourceIdentityVotesWithProof(query);

      expect(wasmSdk.getContestedResourceIdentityVotesWithProofInfo).to.be.calledOnceWithExactly(query);
    });

    it('votePollsByEndDate() fetches active vote polls within a time range', async () => {
      const query = {
        startTimeMs: 1704067200000, // 2024-01-01 00:00:00 UTC
        startTimeIncluded: true,
        endTimeMs: 1706745600000, // 2024-02-01 00:00:00 UTC
        endTimeIncluded: false,
        limit: 100,
        offset: 0,
        orderAscending: true,
      };

      await client.voting.votePollsByEndDate(query);

      expect(wasmSdk.getVotePollsByEndDate).to.be.calledOnceWithExactly(query);
    });

    it('votePollsByEndDateWithProof() fetches vote polls with proof', async () => {
      const query = {
        startTimeMs: 1704067200000,
        endTimeMs: 1706745600000,
        limit: 50,
        offset: 10,
        orderAscending: false,
      };

      await client.voting.votePollsByEndDateWithProof(query);

      expect(wasmSdk.getVotePollsByEndDateWithProofInfo).to.be.calledOnceWithExactly(query);
    });
  });

  describe('Transition Methods', () => {
    it('masternodeVote() casts a vote on a contested resource', async () => {
      const options = {
        masternodeProTxHash,
        contractId: dataContractId,
        documentTypeName: 'domain',
        indexName: 'parentNameAndLabel',
        indexValues: ['dash', 'alice'],
        voteChoice: 'yes',
        signer,
      };

      await client.voting.masternodeVote(options);

      expect(wasmSdk.masternodeVote).to.be.calledOnceWithExactly(options);
    });

    it('masternodeVote() supports different vote choices', async () => {
      // Test abstain vote
      const abstainOptions = {
        masternodeProTxHash,
        contractId: dataContractId,
        documentTypeName: 'domain',
        indexName: 'parentNameAndLabel',
        indexValues: ['dash', 'charlie'],
        voteChoice: 'abstain',
        signer,
      };

      await client.voting.masternodeVote(abstainOptions);

      expect(wasmSdk.masternodeVote).to.be.calledWithExactly(abstainOptions);
    });

    it('masternodeVote() supports lock vote choice', async () => {
      const lockOptions = {
        masternodeProTxHash,
        contractId: dataContractId,
        documentTypeName: 'domain',
        indexName: 'parentNameAndLabel',
        indexValues: ['dash', 'disputed'],
        voteChoice: 'lock',
        signer,
      };

      await client.voting.masternodeVote(lockOptions);

      expect(wasmSdk.masternodeVote).to.be.calledWithExactly(lockOptions);
    });
  });
});
