const BlsSignatures = require('@dashevo/bls');

const createPlatformNodeKeyInput = require('../../../../prompts/createPlatformNodeKeyInput');
const validateBLSPrivateKeyFactory = require('../../../../prompts/validators/validateBLSPrivateKeyFactory');

/**
 *
 * @param {createIpAndPortsForm} createIpAndPortsForm
 * @return {registerMasternodeWithDMTFactory}
 */
function registerMasternodeWithDMTFactory(createIpAndPortsForm) {
  /**
   * Print prompts to collect masternode registration data with DMT
   *
   * @param {Context} ctx
   * @param {TaskWrapper} task
   * @returns {Promise<{
   *   ipAndPorts: {
   *      platformP2PPort: null,
   *      coreP2PPort: null,
   *      platformHTTPPort: null
   *   },
   *   operator: {
   *      rewardShare: null,
   *      privateKey: null
   *   },
   *   platformNodeKey: null
   * }>}
   */
  async function registerMasternodeWithDMT(ctx, task) {
    const blsSignatures = await BlsSignatures();
    const validateBLSPrivateKey = validateBLSPrivateKeyFactory(blsSignatures);

    const prompts = [
      {
        type: 'confirm',
        header: `  Complete initial DMT setup and return here to continue:

    See https://docs.dash.org/dmt-setup for instructions on using Dash Masternode Tool
    to store your collateral and register your masternode.\n`,
        message: 'Press any key to continue dashmate setup process...',
        default: ' ',
        separator: () => '',
        format: () => '',
        initial: true,
        isTrue: () => true,
      },
      {
        type: 'input',
        name: 'operatorPrivateKey',
        header: `  Dashmate needs to collect details on the operator key. The operator
  key is a BLS private key, encoded in hexadecimal format. Dashmate will record the
  private key in the masternode configuration.\n`,
        message: 'Enter masternode operator private key:',
        validate: validateBLSPrivateKey,
      },
    ];

    if (ctx.isHP) {
      prompts.push(createPlatformNodeKeyInput({
        initial: '',
      }));
    }

    prompts.push(await createIpAndPortsForm(ctx.preset, {
      isHPMN: ctx.isHP,
      initialIp: '',
      initialCoreP2PPort: '',
      initialPlatformP2PPort: '',
      initialPlatformHTTPPort: '',
    }));

    const state = await task.prompt(prompts);

    // Keep compatibility with other registration methods
    state.operator = {
      privateKey: state.operatorPrivateKey,
    };

    delete state.operatorPrivateKey;

    return state;
  }

  return registerMasternodeWithDMT;
}

module.exports = registerMasternodeWithDMTFactory;
