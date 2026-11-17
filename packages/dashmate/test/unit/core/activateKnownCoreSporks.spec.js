import fs from 'fs';
import path from 'path';
import { PACKAGE_ROOT_DIR } from '../../../src/constants.js';
import activateKnownCoreSporks from '../../../src/core/activateKnownCoreSporks.js';

const LOCAL_SPORKS = [
  'SPORK_2_INSTANTSEND_ENABLED',
  'SPORK_3_INSTANTSEND_BLOCK_FILTERING',
  'SPORK_9_SUPERBLOCKS_ENABLED',
  'SPORK_17_QUORUM_DKG_ENABLED',
  'SPORK_19_CHAINLOCKS_ENABLED',
];

function readSporkShow(coreVersion) {
  return JSON.parse(fs.readFileSync(
    path.join(PACKAGE_ROOT_DIR, `test/fixtures/coreRpc/${coreVersion}/spork-show.json`),
    'utf8',
  ));
}

describe('activateKnownCoreSporks', () => {
  let rpcClient;

  beforeEach(function beforeEach() {
    rpcClient = {
      spork: this.sinon.stub(),
      sporkupdate: this.sinon.stub().resolves({ result: 'success' }),
    };
  });

  it('should skip the sporks Core v24 removed', async () => {
    // Recorded from Dash Core 24.0.0-rc.6 on regtest, which answers
    // `sporkupdate SPORK_3_INSTANTSEND_BLOCK_FILTERING 0` with "Invalid spork name"
    rpcClient.spork.withArgs('show').resolves({ result: readSporkShow('24.0.0-rc.6') });

    const activated = await activateKnownCoreSporks(rpcClient, LOCAL_SPORKS);

    expect(activated).to.deep.equal([
      'SPORK_2_INSTANTSEND_ENABLED',
      'SPORK_17_QUORUM_DKG_ENABLED',
      'SPORK_19_CHAINLOCKS_ENABLED',
    ]);
    expect(rpcClient.sporkupdate).to.have.been.calledThrice();
    expect(rpcClient.sporkupdate).to.not.have.been.calledWith('SPORK_3_INSTANTSEND_BLOCK_FILTERING');
    expect(rpcClient.sporkupdate).to.not.have.been.calledWith('SPORK_9_SUPERBLOCKS_ENABLED');
  });

  it('should activate every spork Core v23 still has', async () => {
    // Recorded from Dash Core 23.1.10 (dashpay/dashd:23) on regtest
    rpcClient.spork.withArgs('show').resolves({ result: readSporkShow('23.1.10') });

    const activated = await activateKnownCoreSporks(rpcClient, LOCAL_SPORKS);

    expect(activated).to.deep.equal(LOCAL_SPORKS);
    LOCAL_SPORKS.forEach((spork) => {
      expect(rpcClient.sporkupdate).to.have.been.calledWith(spork, 0);
    });
  });

  it('should still try a spork Core does not list unless v24 removed it', async () => {
    rpcClient.spork.withArgs('show').resolves({ result: readSporkShow('24.0.0-rc.6') });
    rpcClient.sporkupdate.withArgs('SPORK_17_QUORUM_DKG_ENABLD')
      .rejects(new Error('Invalid spork name'));

    await expect(activateKnownCoreSporks(rpcClient, ['SPORK_17_QUORUM_DKG_ENABLD']))
      .to.be.rejectedWith('Invalid spork name');
  });
});
