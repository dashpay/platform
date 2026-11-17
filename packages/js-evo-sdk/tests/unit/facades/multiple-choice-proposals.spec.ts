import type { SinonStub } from 'sinon';
import init, * as wasmSDKPackage from '@dashevo/wasm-sdk';
import { EvoSDK } from '../../../dist/sdk.js';

describe('MultipleChoiceProposalsFacade', () => {
  let wasmSdk: wasmSDKPackage.WasmSdk;
  let client: EvoSDK;

  let buildMultipleChoiceProposalStub: SinonStub;
  let buildMultipleChoiceProposalEditStub: SinonStub;
  let buildMultipleChoiceProposalChoiceStub: SinonStub;
  let getMultipleChoiceProposalProcessingFeeStub: SinonStub;
  let getMultipleChoiceProposalChoiceFundStub: SinonStub;
  let getMultipleChoiceProposalStub: SinonStub;
  let getMultipleChoiceProposalsByOwnerStub: SinonStub;
  let getMultipleChoiceProposalsByCreatedAtStub: SinonStub;
  let getMultipleChoiceProposalVotePollStub: SinonStub;
  let getMultipleChoiceProposalVoteStub: SinonStub;
  let getMultipleChoiceProposalKeptChoiceStub: SinonStub;
  let getMasternodeMultipleChoiceProposalVoteStub: SinonStub;
  let multipleChoiceProposalVoteStub: SinonStub;

  const ownerId = '5mjGWa9mruHnLBht3ntBi8CZ6sNk3hZZsQMgTvgQobjS';
  const proposalId = '4EfA9Jrvv3nnCFdSf7fad59851iiTRZ6Wcu6YVJ4iSeF';
  const choiceId = 'GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec';
  const proTxHash = 'a1b2c3d4e5f6a7b8c9d0e1f2a3b4c5d6e7f8a9b0c1d2e3f4a5b6c7d8e9f0a1b2';

  beforeEach(async function setup() {
    await init();
    const builder = wasmSDKPackage.WasmSdkBuilder.testnet();
    wasmSdk = await builder.build();
    client = EvoSDK.fromWasm(wasmSdk);

    buildMultipleChoiceProposalStub = this.sinon.stub(wasmSdk, 'buildMultipleChoiceProposal').resolves({});
    buildMultipleChoiceProposalEditStub = this.sinon.stub(wasmSdk, 'buildMultipleChoiceProposalEdit').returns({});
    buildMultipleChoiceProposalChoiceStub = this.sinon.stub(wasmSdk, 'buildMultipleChoiceProposalChoice').resolves({});
    getMultipleChoiceProposalProcessingFeeStub = this.sinon.stub(wasmSdk, 'getMultipleChoiceProposalProcessingFee').resolves(20_000_000_000n);
    getMultipleChoiceProposalChoiceFundStub = this.sinon.stub(wasmSdk, 'getMultipleChoiceProposalChoiceFund').resolves(10_000_000_000n);
    getMultipleChoiceProposalStub = this.sinon.stub(wasmSdk, 'getMultipleChoiceProposal').resolves(undefined);
    getMultipleChoiceProposalsByOwnerStub = this.sinon.stub(wasmSdk, 'getMultipleChoiceProposalsByOwner').resolves(new Map());
    getMultipleChoiceProposalsByCreatedAtStub = this.sinon.stub(wasmSdk, 'getMultipleChoiceProposalsByCreatedAt').resolves(new Map());
    getMultipleChoiceProposalVotePollStub = this.sinon.stub(wasmSdk, 'getMultipleChoiceProposalVotePoll').returns({});
    getMultipleChoiceProposalVoteStub = this.sinon.stub(wasmSdk, 'getMultipleChoiceProposalVote').resolves({ choices: new Map() });
    getMultipleChoiceProposalKeptChoiceStub = this.sinon.stub(wasmSdk, 'getMultipleChoiceProposalKeptChoice').resolves(undefined);
    getMasternodeMultipleChoiceProposalVoteStub = this.sinon.stub(wasmSdk, 'getMasternodeMultipleChoiceProposalVote').resolves(undefined);
    multipleChoiceProposalVoteStub = this.sinon.stub(wasmSdk, 'multipleChoiceProposalVote').resolves({});
  });

  describe('build()', () => {
    it('should forward the options to buildMultipleChoiceProposal', async () => {
      const options = {
        ownerId,
        title: 'Which block size?',
        descriptionDocument: { id: choiceId, contractId: ownerId, documentType: 'post' },
        url: 'https://forum.dash.org/t/block-size',
        open: true,
      };
      await client.decisionProposals.multipleChoice.build(options);
      expect(buildMultipleChoiceProposalStub).to.be.calledOnceWithExactly(options);
    });
  });

  describe('buildEdit()', () => {
    it('should forward the proposal and the edit to buildMultipleChoiceProposalEdit', async () => {
      const proposal = {} as wasmSDKPackage.Document;
      const edit = { title: 'Which block size, then?', descriptionDocument: null };
      await client.decisionProposals.multipleChoice.buildEdit(proposal, edit);
      expect(buildMultipleChoiceProposalEditStub).to.be.calledOnceWithExactly(proposal, edit);
    });
  });

  describe('buildChoice()', () => {
    it('should forward the options to buildMultipleChoiceProposalChoice', async () => {
      const options = { ownerId, proposalId, title: 'Double it' };
      await client.decisionProposals.multipleChoice.buildChoice(options);
      expect(buildMultipleChoiceProposalChoiceStub).to.be.calledOnceWithExactly(options);
    });
  });

  describe('processingFee()', () => {
    it('should return the fee getMultipleChoiceProposalProcessingFee gives', async () => {
      const fee = await client.decisionProposals.multipleChoice.processingFee();
      expect(getMultipleChoiceProposalProcessingFeeStub).to.be.calledOnceWithExactly();
      expect(fee).to.equal(20_000_000_000n);
    });
  });

  describe('choiceFund()', () => {
    it('should forward the proposal id to getMultipleChoiceProposalChoiceFund', async () => {
      const fund = await client.decisionProposals.multipleChoice.choiceFund(proposalId);
      expect(getMultipleChoiceProposalChoiceFundStub).to.be.calledOnceWithExactly(proposalId);
      expect(fund).to.equal(10_000_000_000n);
    });
  });

  describe('get()', () => {
    it('should forward the proposal id to getMultipleChoiceProposal', async () => {
      const result = await client.decisionProposals.multipleChoice.get(proposalId);
      expect(getMultipleChoiceProposalStub).to.be.calledOnceWithExactly(proposalId);
      expect(result).to.equal(undefined);
    });
  });

  describe('byOwner()', () => {
    it('should forward the query to getMultipleChoiceProposalsByOwner', async () => {
      const query = { ownerId, startAfter: proposalId };
      const result = await client.decisionProposals.multipleChoice.byOwner(query);
      expect(getMultipleChoiceProposalsByOwnerStub).to.be.calledOnceWithExactly(query);
      expect(result).to.be.instanceOf(Map);
    });
  });

  describe('byCreatedAt()', () => {
    it('should forward the query to getMultipleChoiceProposalsByCreatedAt', async () => {
      const query = { limit: 5 };
      await client.decisionProposals.multipleChoice.byCreatedAt(query);
      expect(getMultipleChoiceProposalsByCreatedAtStub).to.be.calledOnceWithExactly(query);
    });

    it('should list from the start without a query', async () => {
      await client.decisionProposals.multipleChoice.byCreatedAt();
      expect(getMultipleChoiceProposalsByCreatedAtStub).to.be.calledOnceWithExactly(undefined);
    });
  });

  describe('votePoll()', () => {
    it('should forward the proposal id to getMultipleChoiceProposalVotePoll', async () => {
      await client.decisionProposals.multipleChoice.votePoll(proposalId);
      expect(getMultipleChoiceProposalVotePollStub).to.be.calledOnceWithExactly(proposalId);
    });
  });

  describe('voteState()', () => {
    it('should forward the proposal id to getMultipleChoiceProposalVote', async () => {
      const vote = await client.decisionProposals.multipleChoice.voteState(proposalId);
      expect(getMultipleChoiceProposalVoteStub).to.be.calledOnceWithExactly(proposalId);
      expect(vote.choices).to.be.instanceOf(Map);
    });
  });

  describe('keptChoice()', () => {
    it('should forward the proposal id to getMultipleChoiceProposalKeptChoice', async () => {
      await client.decisionProposals.multipleChoice.keptChoice(proposalId);
      expect(getMultipleChoiceProposalKeptChoiceStub).to.be.calledOnceWithExactly(proposalId);
    });
  });

  describe('masternodeVote()', () => {
    it('should forward the masternode and the proposal to getMasternodeMultipleChoiceProposalVote', async () => {
      await client.decisionProposals.multipleChoice.masternodeVote(proTxHash, proposalId);
      expect(getMasternodeMultipleChoiceProposalVoteStub)
        .to.be.calledOnceWithExactly(proTxHash, proposalId);
    });
  });

  describe('vote()', () => {
    it('should forward the options to multipleChoiceProposalVote', async () => {
      const options = {
        masternodeProTxHash: proTxHash,
        proposalId,
        voteChoice: { choiceId },
        votingKey: {} as wasmSDKPackage.IdentityPublicKey,
        signer: {} as wasmSDKPackage.IdentitySigner,
      };
      await client.decisionProposals.multipleChoice.vote(options);
      expect(multipleChoiceProposalVoteStub).to.be.calledOnceWithExactly(options);
    });
  });
});
