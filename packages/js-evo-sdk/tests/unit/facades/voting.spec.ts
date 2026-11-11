import type { SinonStub } from 'sinon';
import init, * as wasmSDKPackage from '@dashevo/wasm-sdk';
import { EvoSDK } from '../../../dist/sdk.js';

describe('VotingFacade', () => {
  let wasmSdk: wasmSDKPackage.WasmSdk;
  let client: EvoSDK;
  let signer: wasmSDKPackage.IdentitySigner;

  // Realistic identifiers
  const dataContractId = 'GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec';
  const identityId = '5mjGWa9mruHnLBht3ntBi8CZ6sNk3hZZsQMgTvgQobjS';
  const contenderId = '6o4vL6YpPjamqnnPNpwNSspYJdhPpzYbXvAJ4PYH7Ack';
  const masternodeProTxHash = 'a1b2c3d4e5f6a7b8c9d0e1f2a3b4c5d6e7f8a9b0c1d2e3f4a5b6c7d8e9f0a1b2';

  // Stub references for type-safe assertions
  let getContestedResourceVoteStateStub: SinonStub;
  let getContestedResourceVoteStateWithProofInfoStub: SinonStub;
  let getContestedResourceIdentityVotesStub: SinonStub;
  let getContestedResourceIdentityVotesWithProofInfoStub: SinonStub;
  let getVotePollsByEndDateStub: SinonStub;
  let getVotePollsByEndDateWithProofInfoStub: SinonStub;
  let masternodeVoteStub: SinonStub;
  let getYesNoVotePollStateStub: SinonStub;
  let getYesNoVotePollStateWithProofInfoStub: SinonStub;
  let getMasternodeYesNoVoteStub: SinonStub;
  let getMasternodeYesNoVoteWithProofInfoStub: SinonStub;
  let recordedVote: wasmSDKPackage.Vote;

  beforeEach(async function setup() {
    await init();
    const builder = wasmSDKPackage.WasmSdkBuilder.testnet();
    wasmSdk = await builder.build();
    client = EvoSDK.fromWasm(wasmSdk);

    // Create mock objects
    signer = Object.create(wasmSDKPackage.IdentitySigner.prototype);

    // Stub query methods
    getContestedResourceVoteStateStub = this.sinon.stub(wasmSdk, 'getContestedResourceVoteState').resolves({
      contenders: new Map(),
      abstainVoteCount: 0,
      lockVoteCount: 0,
    });
    getContestedResourceVoteStateWithProofInfoStub = this.sinon.stub(wasmSdk, 'getContestedResourceVoteStateWithProofInfo').resolves({
      data: {
        contenders: new Map(),
        abstainVoteCount: 0,
        lockVoteCount: 0,
      },
      proof: {},
      metadata: {},
    });
    getContestedResourceIdentityVotesStub = this.sinon.stub(wasmSdk, 'getContestedResourceIdentityVotes').resolves([]);
    getContestedResourceIdentityVotesWithProofInfoStub = this.sinon.stub(wasmSdk, 'getContestedResourceIdentityVotesWithProofInfo').resolves({
      data: [],
      proof: {},
      metadata: {},
    });
    getVotePollsByEndDateStub = this.sinon.stub(wasmSdk, 'getVotePollsByEndDate').resolves([]);
    getYesNoVotePollStateStub = this.sinon.stub(wasmSdk, 'getYesNoVotePollState').resolves(undefined);
    getYesNoVotePollStateWithProofInfoStub = this.sinon.stub(wasmSdk, 'getYesNoVotePollStateWithProofInfo').resolves({
      data: undefined,
      proof: {},
      metadata: {},
    });
    getMasternodeYesNoVoteStub = this.sinon.stub(wasmSdk, 'getMasternodeYesNoVote').resolves({
      choice: 'no',
      timesVoted: 2,
    });
    getMasternodeYesNoVoteWithProofInfoStub = this.sinon.stub(wasmSdk, 'getMasternodeYesNoVoteWithProofInfo').resolves({
      data: undefined,
      proof: {},
      metadata: {},
    });
    getVotePollsByEndDateWithProofInfoStub = this.sinon.stub(wasmSdk, 'getVotePollsByEndDateWithProofInfo').resolves({
      data: [],
      proof: {},
      metadata: {},
    });

    // Stub transition method
    recordedVote = Object.create(wasmSDKPackage.Vote.prototype);
    masternodeVoteStub = this.sinon.stub(wasmSdk, 'masternodeVote').resolves(recordedVote);
  });

  describe('contestedResourceVoteState()', () => {
    it('should fetch vote state for a contested resource', async () => {
      const query = {
        dataContractId,
        documentTypeName: 'domain',
        indexName: 'parentNameAndLabel',
        indexValues: ['dash', 'alice'],
        resultType: 'documentsAndVoteTally',
      };

      await client.voting.contestedResourceVoteState(query);

      expect(getContestedResourceVoteStateStub).to.be.calledOnceWithExactly(query);
    });
  });

  describe('contestedResourceVoteStateWithProof()', () => {
    it('should fetch vote state with proof and pagination', async () => {
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

      expect(getContestedResourceVoteStateWithProofInfoStub).to.be.calledOnceWithExactly(query);
    });
  });

  describe('contestedResourceIdentityVotes()', () => {
    it('should fetch votes cast by an identity', async () => {
      const query = {
        identityId,
        limit: 50,
        startAtVoteId: 'AYjR8WFgxoQqeVqKhFNxQvPsaJSZPvzqDJdw4YrFBEhT',
        orderAscending: true,
      };

      await client.voting.contestedResourceIdentityVotes(query);

      expect(getContestedResourceIdentityVotesStub).to.be.calledOnceWithExactly(query);
    });

    it('should return each vote with its count by vote poll id', async () => {
      const votes = new Map([
        [
          'AYjR8WFgxoQqeVqKhFNxQvPsaJSZPvzqDJdw4YrFBEhT',
          Object.create(wasmSDKPackage.ResourceVoteWithCount.prototype),
        ],
      ]);
      getContestedResourceIdentityVotesStub.resolves(votes);

      const result = await client.voting.contestedResourceIdentityVotes({ identityId });

      expect(result).to.equal(votes);
    });
  });

  describe('contestedResourceIdentityVotesWithProof()', () => {
    it('should fetch identity votes with proof', async () => {
      const query = {
        identityId,
        limit: 25,
        startAtVoteId: 'BYjR8WFgxoQqeVqKhFNxQvPsaJSZPvzqDJdw4YrFBEhT',
        orderAscending: false,
      };

      await client.voting.contestedResourceIdentityVotesWithProof(query);

      expect(getContestedResourceIdentityVotesWithProofInfoStub)
        .to.be.calledOnceWithExactly(query);
    });
  });

  describe('votePollsByEndDate()', () => {
    it('should fetch active vote polls within a time range', async () => {
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

      expect(getVotePollsByEndDateStub).to.be.calledOnceWithExactly(query);
    });
  });

  describe('votePollsByEndDateWithProof()', () => {
    it('should fetch vote polls with proof', async () => {
      const query = {
        startTimeMs: 1704067200000,
        endTimeMs: 1706745600000,
        limit: 50,
        offset: 10,
        orderAscending: false,
      };

      await client.voting.votePollsByEndDateWithProof(query);

      expect(getVotePollsByEndDateWithProofInfoStub).to.be.calledOnceWithExactly(query);
    });
  });

  describe('yes/no vote poll queries', () => {
    const yesNoVotePoll = () => Object.create(wasmSDKPackage.VotePoll.prototype);

    it('should fetch a yes/no vote poll state', async () => {
      const votePoll = yesNoVotePoll();

      const state = await client.voting.yesNoVotePollState(votePoll);

      expect(getYesNoVotePollStateStub).to.be.calledOnceWithExactly(votePoll);
      expect(state).to.equal(undefined);
    });

    it('should fetch a yes/no vote poll state with proof', async () => {
      const votePoll = yesNoVotePoll();

      await client.voting.yesNoVotePollStateWithProof(votePoll);

      expect(getYesNoVotePollStateWithProofInfoStub).to.be.calledOnceWithExactly(votePoll);
    });

    it('should fetch a masternode yes/no vote', async () => {
      const votePoll = yesNoVotePoll();

      const vote = await client.voting.masternodeYesNoVote(masternodeProTxHash, votePoll);

      expect(getMasternodeYesNoVoteStub).to.be.calledOnceWithExactly(masternodeProTxHash, votePoll);
      expect(vote).to.deep.equal({ choice: 'no', timesVoted: 2 });
    });

    it('should fetch a masternode yes/no vote with proof', async () => {
      const votePoll = yesNoVotePoll();

      await client.voting.masternodeYesNoVoteWithProof(masternodeProTxHash, votePoll);

      expect(getMasternodeYesNoVoteWithProofInfoStub).to.be.calledOnceWithExactly(
        masternodeProTxHash,
        votePoll,
      );
    });
  });

  describe('masternodeVote()', () => {
    it('should cast a vote on a contested resource and return the recorded vote', async () => {
      const options = {
        masternodeProTxHash,
        contractId: dataContractId,
        documentTypeName: 'domain',
        indexName: 'parentNameAndLabel',
        indexValues: ['dash', 'alice'],
        voteChoice: 'yes',
        signer,
      };

      const vote = await client.voting.masternodeVote(options);

      expect(masternodeVoteStub).to.be.calledOnceWithExactly(options);
      expect(vote).to.equal(recordedVote);
    });

    it('should support abstain vote choice', async () => {
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

      expect(masternodeVoteStub).to.be.calledWithExactly(abstainOptions);
    });

    it('should support lock vote choice', async () => {
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

      expect(masternodeVoteStub).to.be.calledWithExactly(lockOptions);
    });
  });
});
