import fs from 'fs';
import path from 'path';
import getBaseConfigFactory from '../../../../configs/defaults/getBaseConfigFactory.js';
import { PACKAGE_ROOT_DIR } from '../../../../src/constants.js';
import analyseCoreFactory from '../../../../src/doctor/analyse/analyseCoreFactory.js';
import { SEVERITY } from '../../../../src/doctor/Prescription.js';
import Samples from '../../../../src/doctor/Samples.js';

const BLOCKCHAIN_SYNC_PROBLEM = 'Core is syncing blockchain data. Some node services may be temporarily unresponsive';

/**
 * Read an `mnsync status` response captured from a Dash Core v24.0.3 node
 *
 * @param {string} name
 * @return {Object}
 */
function readMnsyncStatus(name) {
  return JSON.parse(fs.readFileSync(
    path.join(PACKAGE_ROOT_DIR, 'test/fixtures/coreRpc/24.0.3', name),
    'utf8',
  ));
}

describe('analyseCoreFactory', () => {
  let analyseCore;
  let samples;

  beforeEach(() => {
    samples = new Samples();
    samples.setDashmateConfig(getBaseConfigFactory()());
    samples.setServiceInfo('core', 'blockchainInfo', { verificationprogress: 0.42 });

    analyseCore = analyseCoreFactory();
  });

  it('should report a node that is still syncing blocks, with its progress', () => {
    samples.setServiceInfo('core', 'masternodeSyncStatus', readMnsyncStatus('mnsync-status-blockchain.json'));

    const problems = analyseCore(samples);

    expect(problems).to.have.lengthOf(1);
    expect(problems[0].getDescription()).to.equal(BLOCKCHAIN_SYNC_PROBLEM);
    expect(problems[0].getSolution()).to.include('42.0% complete');
    expect(problems[0].getSeverity()).to.equal(SEVERITY.MEDIUM);
  });

  // Core keeps IsSynced false after the blocks are in, until it has also
  // fetched governance data. Telling the operator to wait for a block sync
  // that has finished sends them after the wrong thing.
  it('should report governance sync separately once the blocks are synced', () => {
    samples.setServiceInfo('core', 'masternodeSyncStatus', readMnsyncStatus('mnsync-status-governance.json'));

    const problems = analyseCore(samples);

    expect(problems).to.have.lengthOf(1);
    expect(problems[0].getDescription()).to.not.equal(BLOCKCHAIN_SYNC_PROBLEM);
    expect(problems[0].getDescription()).to.include('governance');
    expect(problems[0].getSeverity()).to.equal(SEVERITY.LOW);
  });

  it('should report nothing for a fully synced node', () => {
    samples.setServiceInfo('core', 'masternodeSyncStatus', readMnsyncStatus('mnsync-status.json'));

    expect(analyseCore(samples)).to.be.empty();
  });
});
