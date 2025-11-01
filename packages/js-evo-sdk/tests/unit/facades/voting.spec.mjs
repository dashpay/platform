import init, * as wasmSDKPackage from '@dashevo/wasm-sdk';
import { EvoSDK } from '../../../dist/sdk.js';

describe('VotingFacade', () => {
  let wasmSdk;
  let client;

  beforeEach(async function setup() {
    await init();
    const builder = wasmSDKPackage.WasmSdkBuilder.testnetTrusted();
    wasmSdk = builder.build();
    client = EvoSDK.fromWasm(wasmSdk);

    this.sinon.stub(wasmSdk, 'getContestedResourceVoteState').resolves('ok');
    this.sinon.stub(wasmSdk, 'getContestedResourceVoteStateWithProofInfo').resolves('ok');
    this.sinon.stub(wasmSdk, 'getContestedResourceIdentityVotes').resolves('ok');
    this.sinon.stub(wasmSdk, 'getContestedResourceIdentityVotesWithProofInfo').resolves('ok');
    this.sinon.stub(wasmSdk, 'getVotePollsByEndDate').resolves('ok');
    this.sinon.stub(wasmSdk, 'getVotePollsByEndDateWithProofInfo').resolves('ok');
    this.sinon.stub(wasmSdk, 'masternodeVote').resolves('ok');
  });

  it('contestedResourceVoteState and related queries forward correctly', async () => {
    await client.voting.contestedResourceVoteState({
      dataContractId: 'c',
      documentTypeName: 'dt',
      indexName: 'i',
      indexValues: ['v1'],
      resultType: 'rt',
    });
    await client.voting.contestedResourceVoteStateWithProof({
      dataContractId: 'c',
      documentTypeName: 'dt',
      indexName: 'i',
      indexValues: ['v1'],
      resultType: 'rt',
      includeLockedAndAbstaining: true,
      startAtContenderId: 's',
      startAtIncluded: true,
      limit: 2,
    });
    await client.voting.contestedResourceIdentityVotes({
      identityId: 'id',
      limit: 3,
      startAtVoteId: 's',
      orderAscending: true,
    });
    await client.voting.contestedResourceIdentityVotesWithProof({
      identityId: 'id',
      limit: 4,
      startAtVoteId: 'p',
      orderAscending: false,
    });
    await client.voting.votePollsByEndDate({
      startTimeMs: 1000,
      startTimeIncluded: true,
      endTimeMs: 2000,
      endTimeIncluded: false,
      limit: 2,
      offset: 1,
      orderAscending: true,
    });
    await client.voting.votePollsByEndDateWithProof({
      startTimeMs: 10,
      endTimeMs: 20,
      limit: 1,
      offset: 0,
      orderAscending: false,
    });
    expect(wasmSdk.getContestedResourceVoteState).to.be.calledOnceWithExactly({
      dataContractId: 'c',
      documentTypeName: 'dt',
      indexName: 'i',
      indexValues: ['v1'],
      resultType: 'rt',
    });
    expect(wasmSdk.getContestedResourceVoteStateWithProofInfo).to.be.calledOnceWithExactly({
      dataContractId: 'c',
      documentTypeName: 'dt',
      indexName: 'i',
      indexValues: ['v1'],
      resultType: 'rt',
      includeLockedAndAbstaining: true,
      startAtContenderId: 's',
      startAtIncluded: true,
      limit: 2,
    });
    expect(wasmSdk.getContestedResourceIdentityVotes).to.be.calledOnceWithExactly({
      identityId: 'id',
      limit: 3,
      startAtVoteId: 's',
      orderAscending: true,
    });
    expect(wasmSdk.getContestedResourceIdentityVotesWithProofInfo).to.be.calledOnceWithExactly({
      identityId: 'id',
      limit: 4,
      startAtVoteId: 'p',
      orderAscending: false,
    });
    expect(wasmSdk.getVotePollsByEndDate).to.be.calledOnceWithExactly({
      startTimeMs: 1000,
      startTimeIncluded: true,
      endTimeMs: 2000,
      endTimeIncluded: false,
      limit: 2,
      offset: 1,
      orderAscending: true,
    });
    expect(wasmSdk.getVotePollsByEndDateWithProofInfo).to.be.calledOnceWithExactly({
      startTimeMs: 10,
      endTimeMs: 20,
      limit: 1,
      offset: 0,
      orderAscending: false,
    });
  });

  it('masternodeVote() stringifies array indexValues and forwards', async () => {
    await client.voting.masternodeVote({
      masternodeProTxHash: 'h', contractId: 'c', documentTypeName: 'dt', indexName: 'i', indexValues: ['x', 'y'], voteChoice: 'yes', votingKeyWif: 'w',
    });
    const { args } = wasmSdk.masternodeVote.firstCall;
    expect(args).to.deep.equal(['h', 'c', 'dt', 'i', JSON.stringify(['x', 'y']), 'yes', 'w']);
  });
});
