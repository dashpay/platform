import type { SinonStub } from 'sinon';
import init, * as wasmSDKPackage from '@dashevo/wasm-sdk';
import { EvoSDK } from '../../../dist/sdk.js';

describe('DecisionProposalsFacade', () => {
  let wasmSdk: wasmSDKPackage.WasmSdk;
  let client: EvoSDK;

  let buildDecisionProposalStub: SinonStub;
  let buildDecisionProposalEditStub: SinonStub;
  let getDecisionProposalStub: SinonStub;
  let getDecisionProposalsByEndDateStub: SinonStub;
  let getDecisionProposalsByOwnerStub: SinonStub;
  let getDecisionProposalsByCreatedAtStub: SinonStub;
  let getDecisionProposalPollIdStub: SinonStub;
  let getDecisionProposalPollStateStub: SinonStub;
  let getMasternodeDecisionProposalVoteStub: SinonStub;
  let decisionProposalVoteStub: SinonStub;

  const ownerId = '5mjGWa9mruHnLBht3ntBi8CZ6sNk3hZZsQMgTvgQobjS';
  const proposalId = '4EfA9Jrvv3nnCFdSf7fad59851iiTRZ6Wcu6YVJ4iSeF';
  const proTxHash = 'a1b2c3d4e5f6a7b8c9d0e1f2a3b4c5d6e7f8a9b0c1d2e3f4a5b6c7d8e9f0a1b2';

  beforeEach(async function setup() {
    await init();
    const builder = wasmSDKPackage.WasmSdkBuilder.testnet();
    wasmSdk = await builder.build();
    client = EvoSDK.fromWasm(wasmSdk);

    buildDecisionProposalStub = this.sinon.stub(wasmSdk, 'buildDecisionProposal').resolves({});
    buildDecisionProposalEditStub = this.sinon.stub(wasmSdk, 'buildDecisionProposalEdit').returns({});
    getDecisionProposalStub = this.sinon.stub(wasmSdk, 'getDecisionProposal').resolves(undefined);
    getDecisionProposalsByEndDateStub = this.sinon.stub(wasmSdk, 'getDecisionProposalsByEndDate').resolves(new Map());
    getDecisionProposalsByOwnerStub = this.sinon.stub(wasmSdk, 'getDecisionProposalsByOwner').resolves(new Map());
    getDecisionProposalsByCreatedAtStub = this.sinon.stub(wasmSdk, 'getDecisionProposalsByCreatedAt').resolves(new Map());
    getDecisionProposalPollIdStub = this.sinon.stub(wasmSdk, 'getDecisionProposalPollId').resolves({});
    getDecisionProposalPollStateStub = this.sinon.stub(wasmSdk, 'getDecisionProposalPollState').resolves(undefined);
    getMasternodeDecisionProposalVoteStub = this.sinon.stub(wasmSdk, 'getMasternodeDecisionProposalVote').resolves(undefined);
    decisionProposalVoteStub = this.sinon.stub(wasmSdk, 'decisionProposalVote').resolves({});
  });

  describe('build()', () => {
    it('should forward the options to buildDecisionProposal', async () => {
      const options = {
        ownerId,
        title: 'Raise the block size?',
        description: 'Yes doubles it; no keeps it.',
        url: 'https://forum.dash.org/t/block-size',
        passPercent: 60,
        minimumTurnoutPercent: 10,
        endDate: 1_700_000_000_000n,
      };
      await client.decisionProposals.build(options);
      expect(buildDecisionProposalStub).to.be.calledOnceWithExactly(options);
    });
  });

  describe('buildEdit()', () => {
    it('should forward the proposal and the edit to buildDecisionProposalEdit', async () => {
      const proposal = {} as wasmSDKPackage.Document;
      const edit = { title: 'Raise the block size to 4 MB?', url: null };
      await client.decisionProposals.buildEdit(proposal, edit);
      expect(buildDecisionProposalEditStub).to.be.calledOnceWithExactly(proposal, edit);
    });
  });

  describe('get()', () => {
    it('should forward the proposal id to getDecisionProposal', async () => {
      const result = await client.decisionProposals.get(proposalId);
      expect(getDecisionProposalStub).to.be.calledOnceWithExactly(proposalId);
      expect(result).to.equal(undefined);
    });
  });

  describe('byEndDate()', () => {
    it('should forward the query to getDecisionProposalsByEndDate', async () => {
      const query = { endDateFrom: 1_700_000_000_000n, limit: 20 };
      const result = await client.decisionProposals.byEndDate(query);
      expect(getDecisionProposalsByEndDateStub).to.be.calledOnceWithExactly(query);
      expect(result).to.be.instanceOf(Map);
    });

    it('should list from the start without a query', async () => {
      await client.decisionProposals.byEndDate();
      expect(getDecisionProposalsByEndDateStub).to.be.calledOnceWithExactly(undefined);
    });
  });

  describe('byOwner()', () => {
    it('should forward the query to getDecisionProposalsByOwner', async () => {
      const query = { ownerId, startAfter: proposalId };
      await client.decisionProposals.byOwner(query);
      expect(getDecisionProposalsByOwnerStub).to.be.calledOnceWithExactly(query);
    });
  });

  describe('byCreatedAt()', () => {
    it('should forward the query to getDecisionProposalsByCreatedAt', async () => {
      const query = { limit: 5 };
      await client.decisionProposals.byCreatedAt(query);
      expect(getDecisionProposalsByCreatedAtStub).to.be.calledOnceWithExactly(query);
    });
  });

  describe('pollId()', () => {
    it('should forward the proposal id to getDecisionProposalPollId', async () => {
      await client.decisionProposals.pollId(proposalId);
      expect(getDecisionProposalPollIdStub).to.be.calledOnceWithExactly(proposalId);
    });
  });

  describe('pollState()', () => {
    it('should forward the proposal id to getDecisionProposalPollState', async () => {
      await client.decisionProposals.pollState(proposalId);
      expect(getDecisionProposalPollStateStub).to.be.calledOnceWithExactly(proposalId);
    });
  });

  describe('masternodeVote()', () => {
    it('should forward the masternode and the proposal to getMasternodeDecisionProposalVote', async () => {
      await client.decisionProposals.masternodeVote(proTxHash, proposalId);
      expect(getMasternodeDecisionProposalVoteStub).to.be.calledOnceWithExactly(proTxHash, proposalId);
    });
  });

  describe('vote()', () => {
    it('should forward the options to decisionProposalVote', async () => {
      const options = {
        masternodeProTxHash: proTxHash,
        proposalId,
        voteChoice: 'yes' as const,
        votingKey: {} as wasmSDKPackage.IdentityPublicKey,
        signer: {} as wasmSDKPackage.IdentitySigner,
      };
      await client.decisionProposals.vote(options);
      expect(decisionProposalVoteStub).to.be.calledOnceWithExactly(options);
    });
  });
});
