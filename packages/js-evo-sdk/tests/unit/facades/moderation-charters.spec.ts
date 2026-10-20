import type { SinonStub } from 'sinon';
import init, * as wasmSDKPackage from '@dashevo/wasm-sdk';
import { EvoSDK } from '../../../dist/sdk.js';

describe('ModerationChartersFacade', () => {
  let wasmSdk: wasmSDKPackage.WasmSdk;
  let client: EvoSDK;

  let getModerationSeatedCharterStub: SinonStub;
  let getModerationSubmittedCharterStub: SinonStub;
  let getModerationTeamStub: SinonStub;
  let getModerationSubmittedChartersStub: SinonStub;
  let getModerationJoinRequestsStub: SinonStub;
  let getModerationPendingResignationRequestsStub: SinonStub;
  let buildModerationJoinRequestStub: SinonStub;
  let buildModerationResignationRequestStub: SinonStub;

  const contractId = 'GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec';
  const charterId = '4EfA9Jrvv3nnCFdSf7fad59851iiTRZ6Wcu6YVJ4iSeF';

  beforeEach(async function setup() {
    await init();
    const builder = wasmSDKPackage.WasmSdkBuilder.testnet();
    wasmSdk = await builder.build();
    client = EvoSDK.fromWasm(wasmSdk);

    getModerationSeatedCharterStub = this.sinon.stub(wasmSdk, 'getModerationSeatedCharter').resolves(undefined);
    getModerationSubmittedCharterStub = this.sinon.stub(wasmSdk, 'getModerationSubmittedCharter').resolves(undefined);
    getModerationTeamStub = this.sinon.stub(wasmSdk, 'getModerationTeam').resolves(undefined);
    getModerationSubmittedChartersStub = this.sinon.stub(wasmSdk, 'getModerationSubmittedCharters').resolves(new Map());
    getModerationJoinRequestsStub = this.sinon.stub(wasmSdk, 'getModerationJoinRequests').resolves(new Map());
    getModerationPendingResignationRequestsStub = this.sinon.stub(wasmSdk, 'getModerationPendingResignationRequests').resolves([]);
    buildModerationJoinRequestStub = this.sinon.stub(wasmSdk, 'buildModerationJoinRequest').resolves({});
    buildModerationResignationRequestStub = this.sinon.stub(wasmSdk, 'buildModerationResignationRequest').resolves({});
  });

  describe('seatedCharter()', () => {
    it('should forward the target contract id to getModerationSeatedCharter', async () => {
      const result = await client.moderationCharters.seatedCharter(contractId);
      expect(getModerationSeatedCharterStub).to.be.calledOnceWithExactly(contractId);
      expect(result).to.equal(undefined);
    });
  });

  describe('submittedCharter()', () => {
    it('should forward the proposal id to getModerationSubmittedCharter', async () => {
      await client.moderationCharters.submittedCharter(charterId);
      expect(getModerationSubmittedCharterStub).to.be.calledOnceWithExactly(charterId);
    });
  });

  describe('team()', () => {
    it('should forward the target contract id to getModerationTeam', async () => {
      await client.moderationCharters.team(contractId);
      expect(getModerationTeamStub).to.be.calledOnceWithExactly(contractId);
    });
  });

  describe('submittedCharters()', () => {
    it('should forward the page query to getModerationSubmittedCharters', async () => {
      const query = { targetContractId: contractId, limit: 20, startAfter: charterId };
      const result = await client.moderationCharters.submittedCharters(query);
      expect(getModerationSubmittedChartersStub).to.be.calledOnceWithExactly(query);
      expect(result).to.be.instanceOf(Map);
    });
  });

  describe('joinRequests()', () => {
    it('should forward the page query to getModerationJoinRequests', async () => {
      const query = { submittedCharterId: charterId };
      await client.moderationCharters.joinRequests(query);
      expect(getModerationJoinRequestsStub).to.be.calledOnceWithExactly(query);
    });
  });

  describe('pendingResignationRequests()', () => {
    it('should forward the charter id to getModerationPendingResignationRequests', async () => {
      const result = await client.moderationCharters.pendingResignationRequests(charterId);
      expect(getModerationPendingResignationRequestsStub).to.be.calledOnceWithExactly(charterId);
      expect(result).to.deep.equal([]);
    });
  });

  describe('buildJoinRequest()', () => {
    it('should forward the options to buildModerationJoinRequest', async () => {
      const options = {
        submittedCharterId: charterId,
        message: 'let me help',
        writer: contractId,
        writerEncryptionKey: {} as wasmSDKPackage.PrivateKey,
      };
      await client.moderationCharters.buildJoinRequest(options);
      expect(buildModerationJoinRequestStub).to.be.calledOnceWithExactly(options);
    });
  });

  describe('buildResignationRequest()', () => {
    it('should forward the options to buildModerationResignationRequest', async () => {
      const options = {
        electedCharterId: charterId,
        message: 'moving on',
        writer: contractId,
        writerEncryptionKey: {} as wasmSDKPackage.PrivateKey,
      };
      await client.moderationCharters.buildResignationRequest(options);
      expect(buildModerationResignationRequestStub).to.be.calledOnceWithExactly(options);
    });
  });
});
